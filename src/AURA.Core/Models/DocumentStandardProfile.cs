namespace Aura.Core.Models;

/// <summary>
/// Replaces the prototype's hard-coded Word font check (README §13). Customers define profiles
/// (Corporate Memo, NTRA Memo, Legal Memo...) and the engine compares a document's actual
/// styles against the active profile instead of a fixed value baked into code.
/// </summary>
public sealed class DocumentStandardProfile
{
    public required string Name { get; init; }

    /// <summary>Keyed by structural role - "title", "subtitle", "body", etc.</summary>
    public IReadOnlyDictionary<string, StyleRequirement> Roles { get; init; } =
        new Dictionary<string, StyleRequirement>();

    public string? Alignment { get; init; }
    public double? LineSpacing { get; init; }
}

public sealed class StyleRequirement
{
    public required string Font { get; init; }
    public required double Size { get; init; }
    public bool Bold { get; init; }
}
