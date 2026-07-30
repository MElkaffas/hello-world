namespace Aura.Core.Policy;

/// <summary>Autonomy ceiling an admin sets per app/rule (README §8).</summary>
public enum AutonomyLevel
{
    Observe = 0,
    Notify = 1,
    Suggest = 2,
    AskAndAct = 3,
    AutoAct = 4,
}

public enum PolicyAction
{
    /// <summary>Confidence below the "suggest" threshold - never surfaced.</summary>
    Suppress,
    Observe,
    Notify,
    Suggest,
    AskAndAct,
    AutoAct,
}
