using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;

namespace Aura.Desktop.Views;

/// <summary>Fullscreen transparent overlay for "Select Area" - drag to draw the monitoring
/// rectangle (README §15). Mirrors the prototype's draw-to-scope interaction.</summary>
public partial class RegionSelectionWindow : Window
{
    private Point? _dragStart;

    /// <summary>The drawn region in screen coordinates, or null if cancelled.</summary>
    public Rect? SelectedRegion { get; private set; }

    public RegionSelectionWindow()
    {
        InitializeComponent();
    }

    private void OnMouseDown(object sender, MouseButtonEventArgs e)
    {
        _dragStart = e.GetPosition(RootCanvas);
        SelectionRect.Visibility = Visibility.Visible;
        Canvas.SetLeft(SelectionRect, _dragStart.Value.X);
        Canvas.SetTop(SelectionRect, _dragStart.Value.Y);
        SelectionRect.Width = 0;
        SelectionRect.Height = 0;
        RootCanvas.CaptureMouse();
    }

    private void OnMouseMove(object sender, MouseEventArgs e)
    {
        if (_dragStart is not Point start || e.LeftButton != MouseButtonState.Pressed)
        {
            return;
        }

        var current = e.GetPosition(RootCanvas);
        var x = Math.Min(start.X, current.X);
        var y = Math.Min(start.Y, current.Y);
        var width = Math.Abs(current.X - start.X);
        var height = Math.Abs(current.Y - start.Y);

        Canvas.SetLeft(SelectionRect, x);
        Canvas.SetTop(SelectionRect, y);
        SelectionRect.Width = width;
        SelectionRect.Height = height;
    }

    private void OnMouseUp(object sender, MouseButtonEventArgs e)
    {
        if (_dragStart is not Point start)
        {
            return;
        }

        RootCanvas.ReleaseMouseCapture();

        var current = e.GetPosition(RootCanvas);
        var localRect = new Rect(start, current);

        if (localRect.Width > 4 && localRect.Height > 4)
        {
            var topLeftScreen = RootCanvas.PointToScreen(localRect.TopLeft);
            SelectedRegion = new Rect(topLeftScreen, new Size(localRect.Width, localRect.Height));
        }

        DialogResult = SelectedRegion is not null;
    }

    private void OnKeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key == Key.Escape)
        {
            SelectedRegion = null;
            DialogResult = false;
        }
    }
}
