using System.Drawing;
using System.Windows.Forms;

namespace Aura.Desktop.Services;

/// <summary>System tray presence for AURA - WPF has no native NotifyIcon, so this wraps the
/// WinForms one (UseWindowsForms is enabled in the csproj for exactly this purpose).</summary>
public sealed class TrayIconService : IDisposable
{
    private readonly NotifyIcon _notifyIcon;

    public event EventHandler? StartStopClicked;
    public event EventHandler? SelectAreaClicked;
    public event EventHandler? OpenControlPanelClicked;
    public event EventHandler<AssistantStyle>? AssistantStyleSelected;
    public event EventHandler<string>? LanguageSelected;
    public event EventHandler? ExitClicked;

    public TrayIconService()
    {
        _notifyIcon = new NotifyIcon
        {
            Icon = CreateAccentIcon(),
            Text = "AURA - AI Unified Remote Assistant",
            Visible = true,
            ContextMenuStrip = BuildContextMenu(),
        };

        _notifyIcon.DoubleClick += (_, _) => OpenControlPanelClicked?.Invoke(this, EventArgs.Empty);
    }

    private ContextMenuStrip BuildContextMenu()
    {
        var menu = new ContextMenuStrip();

        var startStop = new ToolStripMenuItem("Start / Stop Monitoring");
        startStop.Click += (_, _) => StartStopClicked?.Invoke(this, EventArgs.Empty);
        menu.Items.Add(startStop);

        var selectArea = new ToolStripMenuItem("Select Area…");
        selectArea.Click += (_, _) => SelectAreaClicked?.Invoke(this, EventArgs.Empty);
        menu.Items.Add(selectArea);

        var controlPanel = new ToolStripMenuItem("Open Control Panel");
        controlPanel.Click += (_, _) => OpenControlPanelClicked?.Invoke(this, EventArgs.Empty);
        menu.Items.Add(controlPanel);

        menu.Items.Add(new ToolStripSeparator());

        var styleMenu = new ToolStripMenuItem("Assistant Style");
        foreach (var style in Enum.GetValues<AssistantStyle>())
        {
            var item = new ToolStripMenuItem(style.ToString());
            item.Click += (_, _) => AssistantStyleSelected?.Invoke(this, style);
            styleMenu.DropDownItems.Add(item);
        }
        menu.Items.Add(styleMenu);

        var languageMenu = new ToolStripMenuItem("Language");
        var english = new ToolStripMenuItem("English");
        english.Click += (_, _) => LanguageSelected?.Invoke(this, "en");
        var arabic = new ToolStripMenuItem("العربية");
        arabic.Click += (_, _) => LanguageSelected?.Invoke(this, "ar");
        languageMenu.DropDownItems.Add(english);
        languageMenu.DropDownItems.Add(arabic);
        menu.Items.Add(languageMenu);

        menu.Items.Add(new ToolStripSeparator());

        var exit = new ToolStripMenuItem("Exit");
        exit.Click += (_, _) => ExitClicked?.Invoke(this, EventArgs.Empty);
        menu.Items.Add(exit);

        return menu;
    }

    private static Icon CreateAccentIcon()
    {
        using var bitmap = new Bitmap(32, 32);
        using (var g = Graphics.FromImage(bitmap))
        {
            g.Clear(Color.Transparent);
            using var brush = new SolidBrush(ColorTranslator.FromHtml("#5980A6"));
            g.FillEllipse(brush, 2, 2, 28, 28);
            using var pen = new Pen(ColorTranslator.FromHtml("#1D2D3D"), 2);
            g.DrawEllipse(pen, 2, 2, 28, 28);
        }

        var handle = bitmap.GetHicon();
        return Icon.FromHandle(handle);
    }

    public void Dispose()
    {
        _notifyIcon.Visible = false;
        _notifyIcon.Dispose();
    }
}
