using System.Text.Json;

namespace Aura.Core.Audit;

using Aura.Core;

/// <summary>Append-only JSON-lines audit log on disk - simple, tamper-evident enough for V0.3-V0.4.</summary>
public sealed class FileAuditLog : IAuditLog
{
    private readonly string _filePath;
    private readonly SemaphoreSlim _lock = new(1, 1);

    public FileAuditLog(string filePath)
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
}
