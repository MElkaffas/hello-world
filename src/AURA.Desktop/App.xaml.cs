using System.Windows;
using Aura.Desktop.Services;

namespace Aura.Desktop;

public partial class App : Application
{
    private AssistantController? _controller;

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);

        _controller = new AssistantController();
        _controller.Start();
    }

    protected override void OnExit(ExitEventArgs e)
    {
        _controller?.Dispose();
        base.OnExit(e);
    }
}
