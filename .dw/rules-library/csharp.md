# Baseline rules — C# / .NET (extends common)

## Style
- Nullable reference types enabled; warnings as errors. `record`/`readonly` for value types.
- `async`/`await` end-to-end; never `.Result`/`.Wait()` (deadlocks). Thread `CancellationToken` through.
- `using` declarations for `IDisposable`; dependency injection over `new` for services.

## Errors
- Catch specific exceptions; never empty catch. Don't use exceptions for normal control flow.
- Validate arguments at public boundaries (`ArgumentNullException.ThrowIfNull`).

## Testing
- xUnit/NUnit; test public behavior. Mock owned boundaries, not the system under test itself.
- Arrange-Act-Assert; one behavior per test.

## Security
- Parameterized queries / EF Core — never concatenate SQL. Validate/encode at boundaries.
- Secrets via configuration / user-secrets / Key Vault, never in source.
