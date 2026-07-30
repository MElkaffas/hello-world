using Aura.Core.Confidence;
using Aura.Core.Models;

namespace Aura.Core.Policy;

/// <summary>
/// Combines the rule's requested autonomy with the admin-configured ceiling for the app/rule,
/// gated by the Confidence Engine's band (README §8-9). Never auto-mutates state itself -
/// it only decides what the pipeline is allowed to do next.
/// </summary>
public sealed class PolicyEngine : IPolicyEngine
{
    private readonly List<PolicyRule> _policies = new();

    public void Configure(IEnumerable<PolicyRule> policyRules)
    {
        _policies.Clear();
        _policies.AddRange(policyRules);
    }

    public PolicyDecision Decide(Finding finding, ConfidenceBand band)
    {
        if (band == ConfidenceBand.Suppress)
        {
            return new PolicyDecision
            {
                Finding = finding,
                EffectiveLevel = AutonomyLevel.Observe,
                ConfidenceBand = band,
                Action = PolicyAction.Suppress,
            };
        }

        var ceiling = ResolveCeiling(finding.Context.App, finding.Rule.Name);
        var requested = ParseAutonomy(finding.Rule.Autonomy);
        var effective = (AutonomyLevel)Math.Min((int)ceiling, (int)requested);

        return new PolicyDecision
        {
            Finding = finding,
            EffectiveLevel = effective,
            ConfidenceBand = band,
            Action = MapAction(effective),
        };
    }

    private AutonomyLevel ResolveCeiling(string app, string ruleName)
    {
        var specific = _policies.FirstOrDefault(p =>
            string.Equals(p.App, app, StringComparison.OrdinalIgnoreCase) &&
            string.Equals(p.RuleName, ruleName, StringComparison.OrdinalIgnoreCase));
        if (specific is not null) return specific.Ceiling;

        var appWide = _policies.FirstOrDefault(p =>
            string.Equals(p.App, app, StringComparison.OrdinalIgnoreCase) && p.RuleName is null);
        if (appWide is not null) return appWide.Ceiling;

        var global = _policies.FirstOrDefault(p => p.App == "*");
        return global?.Ceiling ?? AutonomyLevel.Suggest;
    }

    private static AutonomyLevel ParseAutonomy(string autonomy) => autonomy.ToLowerInvariant() switch
    {
        "observe" => AutonomyLevel.Observe,
        "notify" => AutonomyLevel.Notify,
        "suggest" => AutonomyLevel.Suggest,
        "askandact" or "ask&act" or "askact" => AutonomyLevel.AskAndAct,
        "autoact" or "auto-act" => AutonomyLevel.AutoAct,
        _ => AutonomyLevel.Suggest,
    };

    private static PolicyAction MapAction(AutonomyLevel level) => level switch
    {
        AutonomyLevel.Observe => PolicyAction.Observe,
        AutonomyLevel.Notify => PolicyAction.Notify,
        AutonomyLevel.Suggest => PolicyAction.Suggest,
        AutonomyLevel.AskAndAct => PolicyAction.AskAndAct,
        AutonomyLevel.AutoAct => PolicyAction.AutoAct,
        _ => PolicyAction.Suggest,
    };
}
