using System.Windows;
using Aura.Desktop.Services;
using Aura.Host;
using Aura.Host.Configuration;
using Aura.Host.Connectors;

namespace Aura.Desktop;

public partial class App : Application
{
    private AssistantController? _controller;

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);

        var configuration = LoadConfiguration();
        var registry = new ConnectorRegistry(new IConnectorFactory[]
        {
            new MockOutlookConnectorFactory(),
            new MockWordConnectorFactory(),
        });
        var host = AuraHost.Build(configuration, registry);

        _controller = new AssistantController(host);
        _controller.Start();
    }

    protected override void OnExit(ExitEventArgs e)
    {
        _controller?.Dispose();
        base.OnExit(e);
    }

    /// <summary>Loads tenant.json next to the executable if present, otherwise falls back to
    /// the bundled single-tenant default (mock connectors only).</summary>
    private static AuraConfiguration LoadConfiguration()
    {
        var path = Path.Combine(AppContext.BaseDirectory, "tenant.json");
        return File.Exists(path) ? AuraConfigurationLoader.LoadFromFile(path) : AuraConfigurationLoader.Default();
    }
}
