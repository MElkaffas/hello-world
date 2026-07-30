namespace Aura.Core.Policy;

/// <summary>
/// A configured autonomy ceiling for an app (and optionally a specific rule within it).
/// Example (README §8): Outlook -&gt; Ask&amp;Act ceiling; Word Formatting -&gt; Auto-Act ceiling.
/// </summary>
public sealed class PolicyRule
{
    /// <summary>App name this ceiling applies to, or "*" for the global default.</summary>
    public required string App { get; init; }

    /// <summary>Specific rule name, or null to apply to every rule for the app.</summary>
    public string? RuleName { get; init; }

    public AutonomyLevel Ceiling { get; init; } = AutonomyLevel.Suggest;
}
