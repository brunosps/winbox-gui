# C# / .NET Security Patterns

Covers **ASP.NET Core, ASP.NET (Framework), Blazor, Razor Pages, Minimal APIs, Entity Framework Core, ADO.NET, Dapper, Identity, NuGet**. Used by `/dw-secure-audit` as the primary reference when C# is detected in scope.

---

## Framework Detection

| Indicator | Framework |
|-----------|-----------|
| `Microsoft.AspNetCore.*`, `Program.cs` with `WebApplication.CreateBuilder` | ASP.NET Core (6+) |
| `Startup.cs` with `ConfigureServices` + `Configure` | ASP.NET Core (legacy) |
| `System.Web.Mvc`, `Global.asax` | ASP.NET MVC (Framework) |
| `@page` at top of `.razor` / `.cshtml` | Razor Pages / Blazor |
| `Microsoft.EntityFrameworkCore.*` | EF Core |
| `System.Data.SqlClient`, `Microsoft.Data.SqlClient` | ADO.NET |
| `Dapper` namespace | Dapper |
| `Microsoft.AspNetCore.Identity` | ASP.NET Identity |

---

## SQL Injection

### Flag These

```csharp
// FLAG CRITICAL: EF Core FromSqlRaw with string interpolation
var users = ctx.Users.FromSqlRaw($"SELECT * FROM Users WHERE Name = '{userInput}'");

// FLAG CRITICAL: ADO.NET CommandText concatenation
var cmd = new SqlCommand("SELECT * FROM Users WHERE Name = '" + userInput + "'", conn);

// FLAG CRITICAL: Dapper with string interpolation
var users = conn.Query<User>($"SELECT * FROM Users WHERE Id = {userId}");

// FLAG HIGH: Dynamic LINQ with string expressions from input
var result = db.Users.Where(dynamicExpr); // System.Linq.Dynamic.Core with user string
```

### Safe Patterns

```csharp
// SAFE: EF Core FromSqlInterpolated (binds parameters)
var users = ctx.Users.FromSqlInterpolated($"SELECT * FROM Users WHERE Name = {userInput}");

// SAFE: EF Core LINQ (always parameterized)
var users = ctx.Users.Where(u => u.Name == userInput).ToList();

// SAFE: ADO.NET parameterized
using var cmd = new SqlCommand("SELECT * FROM Users WHERE Name = @name", conn);
cmd.Parameters.Add("@name", SqlDbType.NVarChar).Value = userInput;

// SAFE: Dapper parameterized
var users = conn.Query<User>("SELECT * FROM Users WHERE Id = @Id", new { Id = userId });
```

---

## XSS

### Flag These (Razor / Blazor)

```csharp
// FLAG CRITICAL: @Html.Raw with user input
@Html.Raw(Model.Description)

// FLAG CRITICAL: MvcHtmlString from user input
new MvcHtmlString(userInput)

// FLAG CRITICAL: Blazor MarkupString from user input
<div>@((MarkupString)userInput)</div>

// FLAG CRITICAL: Response.Write without encoding (legacy)
Response.Write(userInput);

// FLAG HIGH: @Html.DisplayFor on already-HTML content without explicit encoding
// Usually safe, but check custom templates that call Html.Raw internally

// FLAG HIGH: IHtmlContent built from user input without HtmlEncoder
new HtmlString(userInput)
```

### Safe Patterns

```csharp
// SAFE: Razor auto-encodes @Expression
<div>@Model.Description</div>        // encoded

// SAFE: explicit encoding when needed
@Html.Encode(userInput)
@System.Net.WebUtility.HtmlEncode(userInput)

// SAFE: Blazor @text interpolation encodes by default
<div>@userInput</div>
```

### Anti-Forgery (CSRF)

```csharp
// FLAG CRITICAL: Controller action modifies state without [ValidateAntiForgeryToken]
[HttpPost]
public IActionResult UpdateProfile(ProfileDto dto) { /* ... */ }

// FLAG HIGH: [IgnoreAntiforgeryToken] on sensitive endpoint
[IgnoreAntiforgeryToken] [HttpPost] public IActionResult DeleteAccount() { /* ... */ }

// FLAG HIGH: Global AntiForgery disabled
services.AddControllers().ConfigureApiBehaviorOptions(o => { /* ... */ });
// plus no [AutoValidateAntiforgeryToken] filter

// SAFE
[HttpPost, ValidateAntiForgeryToken]
public IActionResult UpdateProfile(ProfileDto dto) { /* ... */ }

// Or globally (Razor Pages + MVC)
services.AddMvc(o => o.Filters.Add(new AutoValidateAntiforgeryTokenAttribute()));
```

---

## Deserialization

### Always Flag

```csharp
// FLAG CRITICAL: BinaryFormatter — deprecated, known RCE gadget chains
var bf = new BinaryFormatter();
var obj = bf.Deserialize(stream); // arbitrary type instantiation

// FLAG CRITICAL: NetDataContractSerializer — same class of vulnerability
var ndcs = new NetDataContractSerializer();
ndcs.ReadObject(stream);

// FLAG CRITICAL: LosFormatter (ViewState) with untrusted input
var los = new LosFormatter();
los.Deserialize(userSerialized);

// FLAG CRITICAL: JavaScriptSerializer with type info (pre-netcore)
new JavaScriptSerializer(new SimpleTypeResolver()).Deserialize<object>(userInput);

// FLAG CRITICAL: Newtonsoft.Json with TypeNameHandling != None
var obj = JsonConvert.DeserializeObject<T>(userJson, new JsonSerializerSettings {
  TypeNameHandling = TypeNameHandling.All  // or .Objects / .Auto
});

// FLAG HIGH: System.Text.Json with TypeInfoResolver accepting arbitrary $type

// FLAG HIGH: XmlSerializer with polymorphic types from user XML
```

### Safe Patterns

```csharp
// SAFE: System.Text.Json with closed-type deserialization
var obj = JsonSerializer.Deserialize<UserDto>(userJson); // concrete type only

// SAFE: Newtonsoft with TypeNameHandling.None (default)
var obj = JsonConvert.DeserializeObject<UserDto>(userJson);

// SAFE: DataContractSerializer with KnownTypes allowlist
[DataContract]
[KnownType(typeof(UserDto))]
public abstract class EntityDto { }
```

---

## Authentication & Authorization

### Flag These

```csharp
// FLAG CRITICAL: [AllowAnonymous] on sensitive controller
[AllowAnonymous] [HttpPost("admin/reset-all-passwords")]
public IActionResult ResetAll() { /* ... */ }

// FLAG CRITICAL: missing [Authorize] on controller and global auth not configured
public class AdminController : Controller { public IActionResult Delete(int id) { } }

// FLAG HIGH: Cookie auth without secure flags
services.AddAuthentication(CookieAuthenticationDefaults.AuthenticationScheme)
  .AddCookie(options => {
    options.Cookie.SecurePolicy = CookieSecurePolicy.None; // FLAG
    options.Cookie.HttpOnly = false;                        // FLAG
    options.Cookie.SameSite = SameSiteMode.None;            // FLAG without Secure=true
  });

// FLAG HIGH: JWT validation without issuer / audience / lifetime
new TokenValidationParameters {
  ValidateIssuer = false,       // FLAG
  ValidateAudience = false,     // FLAG
  ValidateLifetime = false,     // FLAG CRITICAL
  ValidateIssuerSigningKey = false, // FLAG CRITICAL
  RequireExpirationTime = false, // FLAG
}

// FLAG HIGH: Password hashing with weak algorithm
var hash = BitConverter.ToString(MD5.HashData(Encoding.UTF8.GetBytes(pass)));

// FLAG HIGH: Identity with weak password policy
services.Configure<IdentityOptions>(o => {
  o.Password.RequiredLength = 4;       // FLAG
  o.Password.RequireDigit = false;
  o.Password.RequireNonAlphanumeric = false;
  o.Lockout.MaxFailedAccessAttempts = 1000; // FLAG
});

// FLAG HIGH: IDOR — no ownership check
public IActionResult GetInvoice(int id) {
  return Ok(db.Invoices.Find(id)); // no userId == invoice.OwnerId check
}
```

### Safe Baselines

```csharp
// Global auth requirement (fallback)
services.AddAuthorization(o => {
  o.FallbackPolicy = new AuthorizationPolicyBuilder().RequireAuthenticatedUser().Build();
});

// Cookie hardening
options.Cookie.HttpOnly = true;
options.Cookie.SecurePolicy = CookieSecurePolicy.Always;
options.Cookie.SameSite = SameSiteMode.Strict;
options.ExpireTimeSpan = TimeSpan.FromMinutes(30);
options.SlidingExpiration = true;

// Password hashing — use ASP.NET Identity's PasswordHasher (PBKDF2) or BCrypt.Net-Next
```

---

## Secrets

### Flag These

```csharp
// FLAG CRITICAL: hardcoded connection string with credentials
services.AddDbContext<AppDb>(o => o.UseSqlServer("Server=x;User=sa;Password=P@ss1;"));

// FLAG CRITICAL: appsettings.json in git with Production secrets
// (scan for: "ConnectionStrings", "ApiKey", "Secret", "Token" in appsettings*.json)

// FLAG HIGH: secret in logs
logger.LogInformation("Connecting with {cs}", builder.ConnectionString);

// FLAG HIGH: Environment variable accessed without guard
var key = Environment.GetEnvironmentVariable("STRIPE_SECRET"); // returns null silently if missing
```

### Safe Patterns

```csharp
// User Secrets (dev only)
// dotnet user-secrets set "Stripe:SecretKey" "sk_test_..."

// Azure Key Vault / AWS Secrets Manager
builder.Configuration.AddAzureKeyVault(new Uri(vaultUri), new DefaultAzureCredential());

// Options pattern with validation
services.AddOptions<StripeOptions>()
  .Bind(builder.Configuration.GetSection("Stripe"))
  .ValidateDataAnnotations()
  .ValidateOnStart();
```

---

## SSRF

```csharp
// FLAG CRITICAL: HttpClient with attacker-controlled URL
var resp = await http.GetStringAsync(userUrl);

// FLAG CRITICAL: WebRequest.Create with user input
var req = WebRequest.Create(userUrl); // deprecated + SSRF

// FLAG HIGH: Redirect on user-controlled URL (open redirect)
return Redirect(userUrl); // Unvalidated redirect

// SAFE: allowlist-based fetch
var host = new Uri(userUrl).Host;
if (!allowedHosts.Contains(host)) throw new SecurityException("host not allowed");
var resp = await http.GetStringAsync(userUrl);

// SAFE: LocalRedirect prevents external redirection
return LocalRedirect(userUrl); // throws if absolute URL
```

---

## Path Traversal

```csharp
// FLAG CRITICAL: direct user path
var content = File.ReadAllText(userPath);

// FLAG CRITICAL: Path.Combine without containment
var full = Path.Combine(baseDir, userFile); // ../../../etc/passwd still works

// FLAG HIGH: Server.MapPath with user input (ASP.NET Framework)
var full = Server.MapPath(userRelative);

// SAFE: canonicalize + contained check
var canonical = Path.GetFullPath(Path.Combine(baseDir, userFile));
if (!canonical.StartsWith(Path.GetFullPath(baseDir) + Path.DirectorySeparatorChar))
  throw new UnauthorizedAccessException("path traversal");
```

---

## Cryptography

### Flag These

```csharp
// FLAG CRITICAL: MD5 / SHA1 for passwords or security tokens
using var md5 = MD5.Create();
var hash = md5.ComputeHash(bytes);

// FLAG CRITICAL: Random for security tokens / session ids
var id = new Random().Next();

// FLAG HIGH: low PBKDF2 iterations
var kdf = new Rfc2898DeriveBytes(password, salt, iterations: 1000); // too low (2026 baseline ≥ 600k for SHA-256)

// FLAG HIGH: DES / 3DES / ECB mode
var aes = Aes.Create();
aes.Mode = CipherMode.ECB; // FLAG

// FLAG HIGH: hardcoded IV or predictable IV
var iv = new byte[16]; // zeros
```

### Safe Patterns

```csharp
// SAFE: cryptographic RNG
var bytes = new byte[32];
RandomNumberGenerator.Fill(bytes);
var token = Convert.ToHexString(bytes);

// SAFE: modern PBKDF2
var kdf = new Rfc2898DeriveBytes(password, salt, 600_000, HashAlgorithmName.SHA256);

// Or BCrypt.Net-Next / Argon2 (via Konscious.Security.Cryptography)
var hash = BCrypt.Net.BCrypt.HashPassword(password, workFactor: 12);

// AES with CBC/GCM + random IV per message
aes.Mode = CipherMode.CBC;
aes.GenerateIV();
```

---

## Supply Chain (NuGet)

### Flag These

- `PackageReference` with floating versions on security-sensitive deps: `<PackageReference Include="Newtonsoft.Json" Version="*" />`
- Missing `packages.lock.json` in CI with `--locked-mode`
- Private feeds configured without authentication — confusion attack surface
- Old `Newtonsoft.Json` versions (< 13.0.1) — known CVE classes
- Abandoned / archived packages still in use

### Safe Patterns

```xml
<!-- Directory.Build.props -->
<PropertyGroup>
  <RestoreLockedMode>true</RestoreLockedMode>
  <RestorePackagesWithLockFile>true</RestorePackagesWithLockFile>
</PropertyGroup>

<!-- .csproj -->
<ItemGroup>
  <PackageReference Include="Microsoft.AspNetCore.Authentication.JwtBearer" Version="8.0.12" />
</ItemGroup>
```

Run `dotnet list package --vulnerable --include-transitive` as part of CI.

---

## ASP.NET Core Middleware Order

Middleware order matters for security. Flag if order is wrong:

```csharp
var app = builder.Build();

// SAFE BASELINE ORDER
app.UseHttpsRedirection();
app.UseHsts();              // only in Production
app.UseStaticFiles();
app.UseRouting();
app.UseCors(policy);        // after UseRouting, before Authentication
app.UseAuthentication();
app.UseAuthorization();
app.UseAntiforgery();       // MVC/Razor Pages
app.MapControllers();

// FLAG HIGH: Authorization before Authentication
app.UseAuthorization();
app.UseAuthentication();    // too late

// FLAG HIGH: UseCors(allow-anything) in Production
app.UseCors(p => p.AllowAnyOrigin().AllowAnyHeader().AllowAnyMethod());

// FLAG HIGH: missing UseHsts + UseHttpsRedirection in Production
```

---

## Minimal APIs

```csharp
// FLAG HIGH: endpoint without [Authorize] or .RequireAuthorization()
app.MapPost("/api/admin/reset", async (Db db) => { /* ... */ });

// FLAG HIGH: model binding without validation
app.MapPost("/users", (UserDto dto, Db db) => db.Users.Add(dto));
// Without [AsParameters] + DataAnnotations or FluentValidation

// SAFE
app.MapPost("/api/admin/reset", Reset).RequireAuthorization("Admin");
```

---

## .NET Remoting / WCF (Legacy)

Flag on sight in modern codebases:

- `System.Runtime.Remoting.*` usage
- WCF `NetTcpBinding` with `TransferMode.Streamed` + `BinaryFormatter` serialization
- `NetDataContractSerializer` anywhere

These have a history of deserialization RCE and are not recommended for new code. Migrate to gRPC / HTTP APIs.

---

## Research Checklist (before flagging)

1. **Is the input attacker-controlled?** — trace from `HttpContext.Request`, `IFormFile`, `[FromBody]`, `[FromQuery]`, external API. Values from `IConfiguration`/env are not attacker-controlled.
2. **Does a validator run upstream?** — check DataAnnotations, FluentValidation, custom `IActionFilter`, Minimal API `IEndpointFilter`.
3. **Is the ORM call parameterized?** — EF Core LINQ and `FromSqlInterpolated` are safe; `FromSqlRaw` with concatenation is not.
4. **Is the auth requirement set globally?** — look for `FallbackPolicy` or `AutoValidateAntiforgeryTokenAttribute` before claiming missing `[Authorize]`.
5. **Is the deserializer type-closed?** — `System.Text.Json.Deserialize<ConcreteType>` without `$type` is safe.
6. **Is the path contained?** — `Path.GetFullPath` + StartsWith-check is the correct pattern.

Only report findings that pass "attacker-controlled input + missing mitigation + exploitable sink".

---

## Grep Patterns

```bash
# SQL injection
grep -rn "FromSqlRaw\|ExecuteSqlRaw" --include="*.cs"
grep -rn "new SqlCommand(.*\+" --include="*.cs"
grep -rn 'conn\.Query.*\${' --include="*.cs"

# XSS
grep -rn "Html\.Raw\|MvcHtmlString\|new HtmlString\|(MarkupString)" --include="*.cs" --include="*.cshtml" --include="*.razor"

# Anti-forgery disabled
grep -rn "IgnoreAntiforgeryToken\|DisableFormValueModelBinding" --include="*.cs"

# Dangerous deserialization
grep -rn "BinaryFormatter\|NetDataContractSerializer\|LosFormatter\|JavaScriptSerializer" --include="*.cs"
grep -rn "TypeNameHandling\s*=\s*TypeNameHandling\.\(All\|Objects\|Auto\)" --include="*.cs"

# Auth anti-patterns
grep -rn "AllowAnonymous" --include="*.cs"
grep -rn "ValidateLifetime\s*=\s*false\|ValidateIssuerSigningKey\s*=\s*false" --include="*.cs"

# Crypto
grep -rn "MD5\.Create\|SHA1\.Create\|new Random(" --include="*.cs"
grep -rn "CipherMode\.ECB" --include="*.cs"
grep -rn "Rfc2898DeriveBytes.*," --include="*.cs" # then inspect iterations

# Path traversal
grep -rn "File\.Read.*Combine\|Server\.MapPath" --include="*.cs"

# Secrets in config
grep -rEn '"(ApiKey|Secret|Password|ConnectionStrings)":\s*"[^"]+' appsettings*.json

# Hardcoded connection strings
grep -rn "UseSqlServer(\"Server=" --include="*.cs"
```

---

## Cross-Reference

For general concepts not specific to C#:
- `../references/xss.md`
- `../references/injection.md`
- `../references/deserialization.md`
- `../references/csrf.md`
- `../references/authentication.md`
- `../references/authorization.md`
- `../references/cryptography.md`
- `../references/supply-chain.md`
- `../references/ssrf.md`
