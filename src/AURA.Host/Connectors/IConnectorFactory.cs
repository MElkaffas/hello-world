using Aura.Core.Actions;
using Aura.Core.Connectors;

namespace Aura.Host.Connectors;

/// <summary>
/// A named, registerable way to construct a connector + (optional) action handler pair. This is
/// the seam a real plugin/manifest model (Phase 5+) would slot into later - today it's just an
/// in-memory registry populated at startup, deliberately not a DLL-loading mechanism yet.
/// </summary>
public interface IConnectorFactory
{
    /// <summary>Registry key referenced by <see cref="Configuration.ConnectorConfig.Implementation"/>,
    /// e.g. "mock-outlook". Independent from the app id the produced connector reports.</summary>
    string Implementation { get; }

    IConnector CreateConnector(IReadOnlyDictionary<string, string> settings);

    /// <summary>Null for read-only connectors (e.g. a Word connector that only scores compliance).</summary>
    IActionHandler? CreateActionHandler(IReadOnlyDictionary<string, string> settings);
}
