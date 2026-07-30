namespace Aura.Core.Security;

public sealed class SensitiveDataMatch
{
    /// <summary>e.g. "email", "nationalId", "creditCard", "phone".</summary>
    public required string Category { get; init; }

    public required string Value { get; init; }
    public int Index { get; init; }
    public int Length { get; init; }
}
