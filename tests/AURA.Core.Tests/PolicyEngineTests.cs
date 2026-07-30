using Aura.Core.Confidence;
using Aura.Core.Models;
using Aura.Core.Policy;
using Xunit;

namespace Aura.Core.Tests;

public class PolicyEngineTests
{
    private static Finding OutlookFinding(string autonomy, string ruleName = "Missing Email Subject")
    {
        var rule = new Rule
        {
            Name = ruleName,
            Trigger = "email.compose",
            Autonomy = autonomy,
        };
        var context = new AuraContext { App = "outlook", Trigger = "email.compose" };
        return new Finding { Rule = rule, Context = context };
    }

    [Fact]
    public void Decide_ReturnsSuppress_WhenConfidenceBandIsSuppress()
    {
        var engine = new PolicyEngine();
        engine.Configure(new[] { new PolicyRule { App = "*", Ceiling = AutonomyLevel.AutoAct } });

        var decision = engine.Decide(OutlookFinding("autoAct"), ConfidenceBand.Suppress);

        Assert.Equal(PolicyAction.Suppress, decision.Action);
    }

    [Fact]
    public void Decide_CapsEffectiveLevel_AtConfiguredCeiling()
    {
        var engine = new PolicyEngine();
        engine.Configure(new[] { new PolicyRule { App = "outlook", Ceiling = AutonomyLevel.Suggest } });

        var decision = engine.Decide(OutlookFinding("autoAct"), ConfidenceBand.Alert);

        Assert.Equal(AutonomyLevel.Suggest, decision.EffectiveLevel);
        Assert.Equal(PolicyAction.Suggest, decision.Action);
    }

    [Fact]
    public void Decide_UsesRuleSpecificCeiling_OverAppWideCeiling()
    {
        var engine = new PolicyEngine();
        engine.Configure(new[]
        {
            new PolicyRule { App = "outlook", Ceiling = AutonomyLevel.Notify },
            new PolicyRule { App = "outlook", RuleName = "Missing Email Subject", Ceiling = AutonomyLevel.AutoAct },
        });

        var decision = engine.Decide(OutlookFinding("autoAct"), ConfidenceBand.Alert);

        Assert.Equal(AutonomyLevel.AutoAct, decision.EffectiveLevel);
        Assert.Equal(PolicyAction.AutoAct, decision.Action);
    }

    [Fact]
    public void Decide_FallsBackToSuggestCeiling_WhenNoPolicyConfigured()
    {
        var engine = new PolicyEngine();
        engine.Configure(Array.Empty<PolicyRule>());

        var decision = engine.Decide(OutlookFinding("autoAct"), ConfidenceBand.Alert);

        Assert.Equal(AutonomyLevel.Suggest, decision.EffectiveLevel);
    }

    [Fact]
    public void Decide_NeverExceedsRuleRequestedAutonomy_EvenIfCeilingIsHigher()
    {
        var engine = new PolicyEngine();
        engine.Configure(new[] { new PolicyRule { App = "outlook", Ceiling = AutonomyLevel.AutoAct } });

        var decision = engine.Decide(OutlookFinding("notify"), ConfidenceBand.Alert);

        Assert.Equal(AutonomyLevel.Notify, decision.EffectiveLevel);
        Assert.Equal(PolicyAction.Notify, decision.Action);
    }
}
