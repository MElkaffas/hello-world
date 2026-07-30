using System.Text.Json.Nodes;

namespace Aura.Core.Models;

/// <summary>
/// A data-driven rule (README §11). Rules are loaded from JSON so a business user can
/// add/edit them without touching code - AURA Core never special-cases an app by name.
/// </summary>
public sealed class Rule
{
    public required string Name { get; init; }

    /// <summary>Dotted event key this rule fires on, e.g. "email.compose", or "*" for any.</summary>
    public required string Trigger { get; init; }

    public IReadOnlyList<RuleCondition> Conditions { get; init; } = Array.Empty<RuleCondition>();

    /// <summary>info | warning | critical</summary>
    public string Severity { get; init; } = "info";

    public LocalizedText? Message { get; init; }

    public SuggestedAction? SuggestedAction { get; init; }

    /// <summary>observe | notify | suggest | askAndAct | autoAct - the autonomy this rule requests (README §8).</summary>
    public string Autonomy { get; init; } = "suggest";

    public bool Audit { get; init; } = true;

    /// <summary>Optional fixed confidence (0-1) overriding the Confidence Engine's heuristic.</summary>
    public double? BaseConfidence { get; init; }
}

public sealed class RuleCondition
{
    /// <summary>Dotted path into the serialized Context, e.g. "email.bodyText" or "document.styles[0].font".</summary>
    public required string Field { get; init; }

    /// <summary>isEmpty | isNotEmpty | equals | notEquals | contains | notContains | greaterThan | lessThan | matchesRegex | in</summary>
    public required string Op { get; init; }

    public JsonNode? Value { get; init; }
}

public sealed class SuggestedAction
{
    /// <summary>e.g. "setField", "insertAttachmentReminder", "applyStyle".</summary>
    public required string Type { get; init; }

    public string? Field { get; init; }

    public JsonNode? Value { get; init; }
}
