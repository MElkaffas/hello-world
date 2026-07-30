namespace Aura.Core.Audit;

public interface IAuditLog
{
    Task RecordAsync(AuditEntry entry, CancellationToken cancellationToken = default);
    Task<IReadOnlyList<AuditEntry>> ReadAllAsync(CancellationToken cancellationToken = default);
}
