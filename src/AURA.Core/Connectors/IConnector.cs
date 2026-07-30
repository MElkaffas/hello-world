using Aura.Core.Models;

namespace Aura.Core.Connectors;

/// <summary>
/// Fills a normalized <see cref="AuraContext"/> from a specific app (README §6). Core never
/// depends on a concrete connector - only on this interface - so it stays app-agnostic.
/// </summary>
public interface IConnector
{
    string App { get; }

    Task<AuraContext> CaptureAsync(CancellationToken cancellationToken = default);
}
