using Aura.Core.Actions;
using Aura.Core.Connectors;
using Aura.Host.Configuration;

namespace Aura.Host.Connectors;

/// <summary>
/// Resolves configured connectors against registered <see cref="IConnectorFactory"/>
/// implementations. Adding a connector today means registering one more factory instance here
/// (or, later, a plugin manifest) - it no longer means editing a UI controller's constructor.
/// </summary>
public sealed class ConnectorRegistry
{
    private readonly Dictionary<string, IConnectorFactory> _factories;

    public ConnectorRegistry(IEnumerable<IConnectorFactory> factories)
    {
        _factories = factories.ToDictionary(f => f.Implementation, StringComparer.OrdinalIgnoreCase);
    }

    public IReadOnlyCollection<string> AvailableImplementations => _factories.Keys;

    public (IConnector Connector, IActionHandler? Handler) Resolve(ConnectorConfig config)
    {
        if (!_factories.TryGetValue(config.Implementation, out var factory))
        {
            throw new InvalidOperationException(
                $"No connector factory registered for implementation '{config.Implementation}'. " +
                $"Available: {string.Join(", ", _factories.Keys)}");
        }

        var connector = factory.CreateConnector(config.Settings);
        var handler = factory.CreateActionHandler(config.Settings);
        return (connector, handler);
    }
}
