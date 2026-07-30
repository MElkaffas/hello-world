using System.Text.Json.Nodes;
using Aura.Core.Connectors;
using Aura.Core.Models;
using Xunit;
using RuleEngineImpl = Aura.Core.Rules.RuleEngine;

namespace Aura.Core.Tests;

public class RuleEngineTests
{
    private static Rule MissingSubjectRule() => new()
    {
        Name = "Missing Email Subject",
        Trigger = "email.compose",
        Conditions = new[]
        {
            new RuleCondition { Field = "email.bodyText", Op = "isNotEmpty" },
            new RuleCondition { Field = "email.subject", Op = "isEmpty" },
        },
        Severity = "warning",
        Message = new LocalizedText { Ar = "نسيت عنوان الرسالة", En = "Subject is empty" },
        SuggestedAction = new SuggestedAction { Type = "setField", Field = "email.subject" },
        Autonomy = "suggest",
    };

    [Fact]
    public async Task Evaluate_FindsMissingSubject_WhenBodyPresentAndSubjectEmpty()
    {
        var engine = new RuleEngineImpl();
        engine.LoadRules(new[] { MissingSubjectRule() });

        var connector = MockConnector.OutlookComposeMissingSubjectAndAttachment();
        var context = await connector.CaptureAsync();

        var findings = engine.Evaluate(context);

        Assert.Single(findings);
        Assert.Equal("Missing Email Subject", findings[0].Rule.Name);
    }

    [Fact]
    public async Task Evaluate_FindsNothing_WhenSubjectIsFilled()
    {
        var engine = new RuleEngineImpl();
        engine.LoadRules(new[] { MissingSubjectRule() });

        var context = new AuraContext
        {
            App = "outlook",
            Trigger = "email.compose",
            Email = new EmailContext { Subject = "Quarterly report", BodyText = "See attached." },
        };

        var findings = engine.Evaluate(context);

        Assert.Empty(findings);
        await Task.CompletedTask;
    }

    [Fact]
    public void Evaluate_IgnoresRule_WhenTriggerDoesNotMatch()
    {
        var engine = new RuleEngineImpl();
        engine.LoadRules(new[] { MissingSubjectRule() });

        var context = new AuraContext
        {
            App = "word",
            Trigger = "document.edit",
        };

        var findings = engine.Evaluate(context);

        Assert.Empty(findings);
    }

    [Fact]
    public void Evaluate_ContainsCondition_MatchesSubstringCaseInsensitively()
    {
        var engine = new RuleEngineImpl();
        engine.LoadRules(new[]
        {
            new Rule
            {
                Name = "Body mentions confidential",
                Trigger = "email.compose",
                Conditions = new[]
                {
                    new RuleCondition { Field = "email.bodyText", Op = "contains", Value = JsonValue.Create("CONFIDENTIAL") },
                },
                Autonomy = "notify",
            },
        });

        var context = new AuraContext
        {
            App = "outlook",
            Trigger = "email.compose",
            Email = new EmailContext { BodyText = "This memo is confidential." },
        };

        var findings = engine.Evaluate(context);

        Assert.Single(findings);
    }

    [Fact]
    public void LoadRulesFromDirectory_ParsesSampleRuleFiles()
    {
        var engine = new RuleEngineImpl();
        var rulesDir = Path.Combine(AppContext.BaseDirectory, "Rules");

        engine.LoadRulesFromDirectory(rulesDir);

        Assert.True(engine.Rules.Count >= 2);
        Assert.Contains(engine.Rules, r => r.Name == "Missing Email Subject");
        Assert.Contains(engine.Rules, r => r.Name == "Attachment Mentioned But Missing");
    }

    [Fact]
    public async Task Evaluate_FindsDocumentOffProfile_UsingComplianceScoreFromDirectoryLoadedRule()
    {
        var engine = new RuleEngineImpl();
        var rulesDir = Path.Combine(AppContext.BaseDirectory, "Rules");
        engine.LoadRulesFromDirectory(rulesDir);

        var connector = MockConnector.WordDocumentOffProfile();
        var context = await connector.CaptureAsync();

        var findings = engine.Evaluate(context);

        Assert.Contains(findings, f => f.Rule.Name == "Document Off Standard Profile");
    }
}
