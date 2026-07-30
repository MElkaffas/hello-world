namespace Aura.Core.Models;

/// <summary>
/// Normalized context produced by a connector + screen understanding, per README §6.
/// The Rule Engine only ever sees this shape - it never knows which app produced it.
/// </summary>
public sealed class AuraContext
{
    public required string App { get; init; }
    public string? Window { get; init; }
    public string Scope { get; init; } = "region";

    /// <summary>
    /// Dotted event key the Rule Engine matches rules against, e.g. "email.compose".
    /// Set by the connector - keeps the Rule Engine app-agnostic (README §6, §11).
    /// </summary>
    public required string Trigger { get; init; }

    public EmailContext? Email { get; init; }
    public DocumentContext? Document { get; init; }

    public DateTimeOffset CapturedAt { get; init; } = DateTimeOffset.UtcNow;
    public bool SensitiveRedacted { get; init; }
}

public sealed class EmailContext
{
    public IReadOnlyList<string> To { get; init; } = Array.Empty<string>();
    public IReadOnlyList<string> Cc { get; init; } = Array.Empty<string>();
    public string Subject { get; init; } = string.Empty;
    public string BodyText { get; init; } = string.Empty;
    public IReadOnlyList<string> Attachments { get; init; } = Array.Empty<string>();
    public bool MentionsAttachment { get; init; }
}

public sealed class DocumentContext
{
    public string? ProfileName { get; init; }
    public IReadOnlyList<DocumentStyleRun> Styles { get; init; } = Array.Empty<DocumentStyleRun>();

    /// <summary>Populated by the connector from <c>DocumentProfileChecker</c> at capture time,
    /// so a plain rule condition ("document.complianceScore lessThan 1.0") can trigger on it
    /// without the Rule Engine needing to know about profiles (README §13).</summary>
    public double? ComplianceScore { get; init; }
    public IReadOnlyList<string> ComplianceIssues { get; init; } = Array.Empty<string>();
}

/// <summary>One paragraph/run in the document, tagged with its structural role (title, body...).</summary>
public sealed class DocumentStyleRun
{
    public required string Role { get; init; }
    public required string Font { get; init; }
    public double Size { get; init; }
    public bool Bold { get; init; }
    public string? Alignment { get; init; }
}
