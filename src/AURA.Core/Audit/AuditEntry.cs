namespace Aura.Core.Audit;

/// <summary>One row of the audit trail: date, user, action, before/after, approved-by (README §12).</summary>
public sealed class AuditEntry
{
    public DateTimeOffset Timestamp { get; init; } = DateTimeOffset.UtcNow;
    public required string App { get; init; }
    public required string RuleName { get; init; }
    public required string Action { get; init; }
    public string? Before { get; init; }
    public string? After { get; init; }
    public string? ApprovedBy { get; init; }
}
