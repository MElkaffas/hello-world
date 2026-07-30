using System.Text.Json;

namespace Aura.Core.Audit;

using Aura.Core;

/// <summary>
/// Default on-prem/single-tenant sink: appends audit entries as JSON-lines to a local file.
/// This is a plain append-only text file - it is NOT encrypted, NOT tamper-evident (no hash
/// chaining or signing), and only safe for single-process access (the lock below is in-process
/// only). Fine as the default for local/on-prem deployments; enterprise/SaaS deployments should
/// implement <see cref="IAuditSink"/> against a real store (DB, API, SIEM) with those properties.
/// </summary>
public sealed class LocalFileAuditSink : IAuditSink
{
    private readonly string _filePath;
    private readonly SemaphoreSlim _lock = new(1, 1);

    public LocalFileAuditSink(string filePath)
    {
        _filePath = filePath;
        var directory = Path.GetDirectoryName(filePath);
        if (!string.IsNullOrEmpty(directory))
        {
            Directory.CreateDirectory(directory);
        }
    }

    public async Task RecordAsync(AuditEntry entry, CancellationToken cancellationToken = default)
    {
        var line = JsonSerializer.Serialize(entry, AuraJson.Options);

        await _lock.WaitAsync(cancellationToken);
        try
        {
            await File.AppendAllTextAsync(_filePath, line + Environment.NewLine, cancellationToken);
        }
        finally
        {
            _lock.Release();
        }
    }

    public async Task<IReadOnlyList<AuditEntry>> ReadAllAsync(CancellationToken cancellationToken = default)
    {
        if (!File.Exists(_filePath))
        {
            return Array.Empty<AuditEntry>();
        }

        await _lock.WaitAsync(cancellationToken);
        try
        {
            var lines = await File.ReadAllLinesAsync(_filePath, cancellationToken);
            return lines
                .Where(l => !string.IsNullOrWhiteSpace(l))
                .Select(l => JsonSerializer.Deserialize<AuditEntry>(l, AuraJson.Options)!)
                .ToList();
        }
        finally
        {
            _lock.Release();
        }
    }

    public async Task DeleteAllAsync(CancellationToken cancellationToken = default)
    {
        await _lock.WaitAsync(cancellationToken);
        try
        {
            if (File.Exists(_filePath))
            {
                File.Delete(_filePath);
            }
        }
        finally
        {
            _lock.Release();
        }
    }
}
