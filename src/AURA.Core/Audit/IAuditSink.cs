namespace Aura.Core.Audit;

/// <summary>
/// Destination for audit entries. Core only depends on this interface - never a concrete
/// storage choice - so a tenant/deployment can point audit output at a local file, SQLite,
/// an enterprise API, or a SIEM without Core or the pipeline changing.
/// </summary>
public interface IAuditSink
{
    Task RecordAsync(AuditEntry entry, CancellationToken cancellationToken = default);
    Task<IReadOnlyList<AuditEntry>> ReadAllAsync(CancellationToken cancellationToken = default);

    /// <summary>Erases all captured audit entries (the Privacy Indicator's "Delete captured
    /// data" action, README §10). Callers should go through this rather than reaching around
    /// the sink for a file path - a DB/API-backed sink has no such path to reach for.</summary>
    Task DeleteAllAsync(CancellationToken cancellationToken = default);
}
