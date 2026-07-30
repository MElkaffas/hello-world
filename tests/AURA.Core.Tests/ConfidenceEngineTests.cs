using Aura.Core.Confidence;
using Aura.Core.Models;
using Xunit;

namespace Aura.Core.Tests;

public class ConfidenceEngineTests
{
    private static Finding FindingWith(params string[] ops)
    {
        var rule = new Rule
        {
            Name = "Test Rule",
            Trigger = "*",
            Conditions = ops.Select(op => new RuleCondition { Field = "email.subject", Op = op }).ToArray(),
            Autonomy = "suggest",
        };
        var context = new AuraContext { App = "outlook", Trigger = "email.compose" };
        return new Finding { Rule = rule, Context = context };
    }

    [Fact]
    public void Score_IsHigh_ForDeterministicConditions()
    {
        var engine = new ConfidenceEngine();
        var finding = FindingWith("isEmpty");

        var score = engine.Score(finding);

        Assert.Equal(0.95, score);
        Assert.Equal(ConfidenceBand.Alert, engine.Band(score));
    }

    [Fact]
    public void Score_IsLower_ForHeuristicConditions()
    {
        var engine = new ConfidenceEngine();
        var finding = FindingWith("contains", "matchesRegex");

        var score = engine.Score(finding);

        Assert.Equal(0.75, score);
        Assert.Equal(ConfidenceBand.Suggest, engine.Band(score));
    }

    [Fact]
    public void Score_UsesBaseConfidenceOverride_WhenSet()
    {
        var engine = new ConfidenceEngine();
        var rule = new Rule
        {
            Name = "Overridden",
            Trigger = "*",
            Conditions = new[] { new RuleCondition { Field = "email.subject", Op = "contains" } },
            Autonomy = "suggest",
            BaseConfidence = 0.5,
        };
        var finding = new Finding { Rule = rule, Context = new AuraContext { App = "outlook", Trigger = "email.compose" } };

        var score = engine.Score(finding);

        Assert.Equal(0.5, score);
        Assert.Equal(ConfidenceBand.Suppress, engine.Band(score));
    }

    [Theory]
    [InlineData(0.95, ConfidenceBand.Alert)]
    [InlineData(0.90, ConfidenceBand.Suggest)]
    [InlineData(0.70, ConfidenceBand.Suggest)]
    [InlineData(0.69, ConfidenceBand.Suppress)]
    [InlineData(0.10, ConfidenceBand.Suppress)]
    public void Band_MapsScoreToThresholds(double score, ConfidenceBand expected)
    {
        var engine = new ConfidenceEngine();

        Assert.Equal(expected, engine.Band(score));
    }
}
