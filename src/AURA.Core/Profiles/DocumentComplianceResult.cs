namespace Aura.Core.Profiles;

public sealed class DocumentComplianceResult
{
    /// <summary>0-1 compliance score against the active profile.</summary>
    public required double Score { get; init; }

    public IReadOnlyList<string> Issues { get; init; } = Array.Empty<string>();
}
