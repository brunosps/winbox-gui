# Recipe: `WebApplicationFactory<T>` + xUnit (.NET)

Use for ASP.NET Core minimal API or MVC. Microsoft's official integration-testing pattern. Runs the full pipeline (DI, middleware, filters) in-process — no Kestrel port, no flake.

## File shape

`{{PRD_PATH}}/QA/scripts/api/RF_XX_[Slug]Tests.cs`

```csharp
using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using Microsoft.AspNetCore.Mvc.Testing;
using Xunit;

namespace YourProject.QA.Api;

public class RF_XX_CreateUserTests : IClassFixture<WebApplicationFactory<Program>>
{
    private readonly WebApplicationFactory<Program> _factory;
    private readonly string _tokenAdmin = Environment.GetEnvironmentVariable("QA_TOKEN_ADMIN") ?? "";
    private readonly string _tokenOtherOrg = Environment.GetEnvironmentVariable("QA_TOKEN_OTHER_ORG") ?? "";

    public RF_XX_CreateUserTests(WebApplicationFactory<Program> factory) => _factory = factory;

    private HttpClient Client(string? token = null)
    {
        var c = _factory.CreateClient();
        if (!string.IsNullOrEmpty(token))
            c.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", token);
        return c;
    }

    private record CreateUserDto(string Email, string Name);
    private record UserResponse(string Id, string Email, string Name, DateTime CreatedAt);

    [Fact]
    public async Task HappyPath_Returns201()
    {
        var r = await Client(_tokenAdmin).PostAsJsonAsync("/users",
            new CreateUserDto($"qa-{Guid.NewGuid():N}@example.com", "QA"));
        Assert.Equal(HttpStatusCode.Created, r.StatusCode);
        var body = await r.Content.ReadFromJsonAsync<UserResponse>();
        Assert.NotNull(body);
        Assert.NotNull(body!.Id);
    }

    [Theory]
    [InlineData("{\"name\":\"No email\"}", "email")]
    [InlineData("{\"email\":\"no-name@x.com\"}", "name")]
    [InlineData("{\"email\":\"not-an-email\",\"name\":\"X\"}", "email")]
    public async Task Validation_Returns422_AndMentionsField(string payload, string field)
    {
        var content = new StringContent(payload, System.Text.Encoding.UTF8, "application/json");
        var r = await Client(_tokenAdmin).PostAsync("/users", content);
        Assert.Equal(HttpStatusCode.UnprocessableEntity, r.StatusCode);
        var msg = await r.Content.ReadAsStringAsync();
        Assert.Contains(field, msg.ToLower());
    }

    [Fact]
    public async Task NoToken_Returns401()
    {
        var r = await Client().PostAsJsonAsync("/users", new CreateUserDto("x@y.com", "x"));
        Assert.Equal(HttpStatusCode.Unauthorized, r.StatusCode);
    }

    [Fact]
    public async Task CrossTenant_Returns403Or404()
    {
        if (string.IsNullOrEmpty(_tokenOtherOrg)) return;
        // assume a known id from another tenant; in a real suite, create one in setup
        var r = await Client(_tokenOtherOrg).GetAsync("/users/00000000-0000-0000-0000-000000000001");
        Assert.True(r.StatusCode is HttpStatusCode.Forbidden or HttpStatusCode.NotFound);
    }

    [Fact]
    public async Task Contract_HasRequiredFields_NoLeaks()
    {
        var create = await Client(_tokenAdmin).PostAsJsonAsync("/users",
            new CreateUserDto($"contract-{Guid.NewGuid():N}@example.com", "Contract"));
        var created = await create.Content.ReadFromJsonAsync<UserResponse>();
        var get = await Client(_tokenAdmin).GetAsync($"/users/{created!.Id}");
        Assert.Equal(HttpStatusCode.OK, get.StatusCode);

        var raw = await get.Content.ReadAsStringAsync();
        foreach (var field in new[] { "id", "email", "name", "created_at" })
            Assert.Contains(field, raw, StringComparison.OrdinalIgnoreCase);
        foreach (var leak in new[] { "password_hash", "internal_id", "_raw" })
            Assert.DoesNotContain(leak, raw, StringComparison.OrdinalIgnoreCase);
    }
}
```

## Configuration

Project file (`*.QA.csproj` or extend the existing test project):

```xml
<ItemGroup>
  <PackageReference Include="Microsoft.AspNetCore.Mvc.Testing" Version="8.0.*" />
  <PackageReference Include="xunit" Version="2.9.*" />
  <PackageReference Include="xunit.runner.visualstudio" Version="2.8.*" />
  <PackageReference Include="Microsoft.NET.Test.Sdk" Version="17.11.*" />
</ItemGroup>
<ItemGroup>
  <InternalsVisibleTo Include="$(AssemblyName)" />
</ItemGroup>
```

The `Program` class must be public (for `WebApplicationFactory<Program>`). For minimal APIs, add at the bottom of `Program.cs`:

```csharp
public partial class Program { }
```

## Running

```bash
# all RF tests
dotnet test --filter FullyQualifiedName~YourProject.QA.Api

# one RF
dotnet test --filter FullyQualifiedName~RF_XX_CreateUserTests

# log to QA/logs/api/
dotnet test --filter FullyQualifiedName~YourProject.QA.Api \
  --logger "console;verbosity=detailed" 2>&1 \
  | tee "QA/logs/api/run-$(date +%F).log"
```

## Logging request/response

Use a custom `DelegatingHandler` registered on the factory's client:

```csharp
public class LoggingHandler : DelegatingHandler
{
    private static readonly string LogPath = "QA/logs/api/RF-XX-create-user.log";

    protected override async Task<HttpResponseMessage> SendAsync(
        HttpRequestMessage req, CancellationToken ct)
    {
        var sw = Stopwatch.StartNew();
        var res = await base.SendAsync(req, ct);
        sw.Stop();
        Directory.CreateDirectory(Path.GetDirectoryName(LogPath)!);
        var entry = new {
            ts = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
            method = req.Method.Method,
            url = req.RequestUri?.ToString(),
            status = (int)res.StatusCode,
            ms = sw.ElapsedMilliseconds,
        };
        await File.AppendAllTextAsync(LogPath,
            System.Text.Json.JsonSerializer.Serialize(entry) + "\n", ct);
        return res;
    }
}
```

## Pros / cons

- **Pro**: in-process — full DI graph, no port, deterministic.
- **Pro**: `[Theory]` + `[InlineData]` covers the 4xx matrix.
- **Pro**: same project as unit tests; `dotnet test` runs both.
- **Con**: requires `Program` to be partial and public.
- **Con**: tied to `Microsoft.AspNetCore.Mvc.Testing` package versions.
