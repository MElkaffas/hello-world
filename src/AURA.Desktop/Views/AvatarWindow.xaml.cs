using System.Windows;
using System.Windows.Input;
using System.Windows.Media.Animation;
using System.Windows.Threading;
using Aura.Desktop.Services;

namespace Aura.Desktop.Views;

/// <summary>
/// The draggable robot avatar (README §14 hi-fi spec): 92x64 head, mouse-tracking pupils,
/// blink, mood mouth, bob/walk animation, antenna pulse on alert.
/// </summary>
public partial class AvatarWindow : Window
{
    private readonly DispatcherTimer _pupilTracker;
    private Storyboard? _activeBodyStoryboard;
    private AssistantMode _mode = AssistantMode.Idle;
    private RobotMood _mood = RobotMood.Idle;
    private AssistantStyle _style = AssistantStyle.Robot;

    public AvatarWindow()
    {
        InitializeComponent();

        _pupilTracker = new DispatcherTimer(DispatcherPriority.Render)
        {
            Interval = TimeSpan.FromMilliseconds(33),
        };
        _pupilTracker.Tick += OnPupilTrackerTick;

        Loaded += OnLoaded;
        Closed += (_, _) => _pupilTracker.Stop();
    }

    private void OnLoaded(object? sender, RoutedEventArgs e)
    {
        ((Storyboard)Resources["BlinkStoryboard"]).Begin(this, isControllable: true);
        _pupilTracker.Start();
        SetMode(_mode);
        SetMood(_mood);
        SetStyle(_style);
    }

    public void SetMode(AssistantMode mode)
    {
        _mode = mode;
        PlayBodyStoryboard(mode == AssistantMode.Watching ? "WalkStoryboard" : "BobStoryboard");
    }

    public void SetMood(RobotMood mood)
    {
        _mood = mood;

        MouthNeutral.Visibility = Visibility.Collapsed;
        MouthAlert.Visibility = Visibility.Collapsed;
        MouthHappy.Visibility = Visibility.Collapsed;

        switch (mood)
        {
            case RobotMood.Alert:
                MouthAlert.Visibility = Visibility.Visible;
                break;
            case RobotMood.Happy:
                MouthHappy.Visibility = Visibility.Visible;
                break;
            default:
                MouthNeutral.Visibility = Visibility.Visible;
                break;
        }

        var pulse = (Storyboard)Resources["PulseStoryboard"];
        if (mood == RobotMood.Alert)
        {
            pulse.Begin(this, isControllable: true);
        }
        else
        {
            pulse.Stop(this);
            AntennaDot.Opacity = 1.0;
        }
    }

    public void SetStyle(AssistantStyle style)
    {
        _style = style;

        switch (style)
        {
            case AssistantStyle.Robot:
                Visibility = Visibility.Visible;
                BodyChip.Visibility = Visibility.Visible;
                break;
            case AssistantStyle.Minimal:
                Visibility = Visibility.Visible;
                BodyChip.Visibility = Visibility.Collapsed;
                break;
            case AssistantStyle.Professional:
            case AssistantStyle.Invisible:
                Visibility = Visibility.Hidden;
                break;
        }
    }

    private void PlayBodyStoryboard(string resourceKey)
    {
        _activeBodyStoryboard?.Stop(this);
        var storyboard = (Storyboard)Resources[resourceKey];
        storyboard.Begin(this, isControllable: true);
        _activeBodyStoryboard = storyboard;
    }

    private void OnRobotMouseDown(object sender, MouseButtonEventArgs e)
    {
        if (e.LeftButton == MouseButtonState.Pressed)
        {
            DragMove();
        }
    }

    private void OnPupilTrackerTick(object? sender, EventArgs e)
    {
        TrackPupil(LeftEye, LeftPupilOffset);
        TrackPupil(RightEye, RightPupilOffset);
    }

    private static void TrackPupil(FrameworkElement eye, System.Windows.Media.TranslateTransform pupilOffset)
    {
        if (!eye.IsVisible || eye.ActualWidth <= 0)
        {
            return;
        }

        Point eyeCenterScreen;
        try
        {
            eyeCenterScreen = eye.PointToScreen(new Point(eye.ActualWidth / 2, eye.ActualHeight / 2));
        }
        catch (InvalidOperationException)
        {
            return;
        }

        var cursor = System.Windows.Forms.Cursor.Position;
        var dx = cursor.X - eyeCenterScreen.X;
        var dy = cursor.Y - eyeCenterScreen.Y;
        var distance = Math.Sqrt(dx * dx + dy * dy);

        const double maxOffset = 4.0;
        if (distance > maxOffset && distance > 0)
        {
            dx = dx / distance * maxOffset;
            dy = dy / distance * maxOffset;
        }

        pupilOffset.X = dx;
        pupilOffset.Y = dy;
    }
}
