using System.Windows;

namespace Aura.Desktop.Views;

/// <summary>
/// The Suggest → Preview → Approve/Ignore notification bubble (README §7), with the
/// multi-alert "Next" counter from the prototype (README §15).
/// </summary>
public partial class AlertBubbleWindow : Window
{
    public event EventHandler? ApplyRequested;
    public event EventHandler? IgnoreRequested;
    public event EventHandler? NextRequested;

    public AlertBubbleWindow()
    {
        InitializeComponent();
    }

    public void Display(string message, string preview, int index, int total, string? status = null)
    {
        MessageText.Text = message;
        PreviewText.Text = preview;
        CounterText.Text = total > 1 ? $"{index} of {total}" : string.Empty;
        NextButton.Visibility = total > 1 && index < total ? Visibility.Visible : Visibility.Collapsed;

        StatusText.Visibility = string.IsNullOrEmpty(status) ? Visibility.Collapsed : Visibility.Visible;
        StatusText.Text = status ?? string.Empty;
    }

    public void SetBusy(bool busy)
    {
        ApplyButton.IsEnabled = !busy;
        IgnoreButton.IsEnabled = !busy;
        NextButton.IsEnabled = !busy;
    }

    private void OnApplyClicked(object sender, RoutedEventArgs e) => ApplyRequested?.Invoke(this, EventArgs.Empty);

    private void OnIgnoreClicked(object sender, RoutedEventArgs e) => IgnoreRequested?.Invoke(this, EventArgs.Empty);

    private void OnNextClicked(object sender, RoutedEventArgs e) => NextRequested?.Invoke(this, EventArgs.Empty);
}
