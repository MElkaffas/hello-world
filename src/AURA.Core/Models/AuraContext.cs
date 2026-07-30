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

    /// <summary>Who/what session this capture belongs to - additive, optional; existing
    /// connectors and rules that only ever set Email/Document keep working unchanged.</summary>
    public UserContext? User { get; init; }
    public SessionContext? Session { get; init; }
    public SecurityContext? Security { get; init; }
    public EnvironmentContext? Environment { get; init; }

    public DateTimeOffset CapturedAt { get; init; } = DateTimeOffset.UtcNow;
    public bool SensitiveRedacted { get; init; }
}

/// <summary>The authenticated principal a capture is attributed to - not yet populated by any
/// connector (no identity provider exists yet); the shape exists now so Rule/Policy/Audit
/// don't need to change again once one does.</summary>
public sealed class UserContext
{
    public string? UserId { get; init; }
    public string? DisplayName { get; init; }
    public IReadOnlyList<string> Roles { get; init; } = Array.Empty<string>();
}

/// <summary>The monitoring session a capture happened within, e.g. for session-scoped
/// permissions like "Monitor Outlook for 30 min" (README §10) - not yet enforced anywhere.</summary>
public sealed class SessionContext
{
    public string? SessionId { get; init; }
    public DateTimeOffset StartedAt { get; init; } = DateTimeOffset.UtcNow;
    public TimeSpan? TimeLimit { get; init; }
}

/// <summary>Tenant/processing-location metadata a capture happened under. <see cref="ProcessingLocation"/>
/// should reflect a real routing decision once an AI Gateway exists - today nothing sets it from
/// live logic, so treat any non-default value here as informational, not a verified guarantee.</summary>
public sealed class SecurityContext
{
    public string? TenantId { get; init; }
    public string ProcessingLocation { get; init; } = "local";
    public bool RedactionApplied { get; init; }
}

/// <summary>Device/config metadata useful for audit and staged policy rollouts.</summary>
public sealed class EnvironmentContext
{
    public string? DeviceId { get; init; }
    public string? OsVersion { get; init; }
    public string? PolicyVersion { get; init; }
    public string? ConfigurationVersion { get; init; }
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
