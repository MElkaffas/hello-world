using System.Windows;
using System.Windows.Controls;
using Aura.Desktop.Services;

namespace Aura.Desktop.Views;

/// <summary>
/// Monitoring + Privacy control surface (README §10, §15): scope picker, Start/Stop,
/// Select Area, Knowledge Base tags, live Privacy Indicator, and an Audit Log viewer.
/// </summary>
public partial class ControlPanelWindow : Window
{
    public event EventHandler<string>? LanguageChanged;
    public event EventHandler<AssistantStyle>? StyleChanged;
    public event EventHandler<MonitoringScope>? ScopeChanged;
    public event EventHandler? SelectAreaRequested;
    public event EventHandler? StartStopRequested;
    public event EventHandler? DeleteDataRequested;
    public event EventHandler? RefreshAuditRequested;

    public ControlPanelWindow()
    {
        InitializeComponent();

        StyleCombo.ItemsSource = Enum.GetValues<AssistantStyle>();
        StyleCombo.SelectedItem = AssistantStyle.Robot;

        ScopeCombo.ItemsSource = Enum.GetValues<MonitoringScope>();
        ScopeCombo.SelectedItem = MonitoringScope.Region;
    }

    public bool IsOutlookScenarioSelected => OutlookRadio.IsChecked == true;

    public void SetStatus(string text) => StatusText.Text = text;

    public void SetPrivacyStatus(string text) => PrivacyStatusText.Text = text;

    public void SetStartStopLabel(string text) => StartStopButton.Content = text;

    public void SetAuditEntries(IEnumerable<string> lines)
    {
        AuditListBox.ItemsSource = lines;
    }

    private void OnLanguageEnClicked(object sender, RoutedEventArgs e) => LanguageChanged?.Invoke(this, "en");

    private void OnLanguageArClicked(object sender, RoutedEventArgs e) => LanguageChanged?.Invoke(this, "ar");

    private void OnStyleChanged(object sender, SelectionChangedEventArgs e)
    {
        if (StyleCombo.SelectedItem is AssistantStyle style)
        {
            StyleChanged?.Invoke(this, style);
        }
    }

    private void OnScopeChanged(object sender, SelectionChangedEventArgs e)
    {
        if (ScopeCombo.SelectedItem is MonitoringScope scope)
        {
            ScopeChanged?.Invoke(this, scope);
        }
    }

    private void OnSelectAreaClicked(object sender, RoutedEventArgs e) => SelectAreaRequested?.Invoke(this, EventArgs.Empty);

    private void OnStartStopClicked(object sender, RoutedEventArgs e) => StartStopRequested?.Invoke(this, EventArgs.Empty);

    private void OnDeleteDataClicked(object sender, RoutedEventArgs e) => DeleteDataRequested?.Invoke(this, EventArgs.Empty);

    private void OnRefreshAuditClicked(object sender, RoutedEventArgs e) => RefreshAuditRequested?.Invoke(this, EventArgs.Empty);
}
