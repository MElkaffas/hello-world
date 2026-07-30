namespace Aura.Desktop.Services;

/// <summary>Assistant presentation modes (README §14): Robot (full body), Minimal (head only),
/// Professional (notification card only, no avatar), Invisible (nothing shown).</summary>
public enum AssistantStyle
{
    Robot,
    Minimal,
    Professional,
    Invisible,
}

public enum AssistantMode
{
    Idle,
    Drawing,
    Watching,
}

public enum RobotMood
{
    Idle,
    Thinking,
    Alert,
    Happy,
}

/// <summary>Monitoring scope options (README §10 - extends the prototype's four options).</summary>
public enum MonitoringScope
{
    Nothing,
    Region,
    Window,
    Application,
    Document,
    UiElement,
    Workflow,
    Task,
}
