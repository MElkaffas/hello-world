namespace Aura.Core.Models;

/// <summary>A rule that matched against a captured context, before confidence scoring.</summary>
public sealed class Finding
{
    public required Rule Rule { get; init; }
    public required AuraContext Context { get; init; }
    public DateTimeOffset DetectedAt { get; init; } = DateTimeOffset.UtcNow;

    /// <summary>Filled in by the Confidence Engine (README §9).</summary>
    public double Confidence { get; set; }
}
