using Aura.Core.Actions;
using Aura.Core.Audit;
using Aura.Core.Confidence;
using Aura.Core.Connectors;
using Aura.Core.Models;
using Aura.Core.Pipeline;
using Aura.Core.Policy;
using Xunit;
using RuleEngineImpl = Aura.Core.Rules.RuleEngine;

namespace Aura.Core.Tests;

public class PipelineIntegrationTests : IDisposable
{
    private readonly string _auditFile = Path.Combine(Path.GetTempPath(), $"aura-audit-{Guid.NewGuid():N}.jsonl");

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

    private DetectionPipeline BuildPipeline(AutonomyLevel outlookCeiling = AutonomyLevel.Suggest)
    {
        var ruleEngine = new RuleEngineImpl();
        ruleEngine.LoadRules(new[] { MissingSubjectRule() });

        var confidenceEngine = new ConfidenceEngine();

        var policyEngine = new PolicyEngine();
        policyEngine.Configure(new[] { new PolicyRule { App = "outlook", Ceiling = outlookCeiling } });

        var auditLog = new FileAuditLog(_auditFile);
        var handler = new MockActionHandler("outlook");

        return new DetectionPipeline(ruleEngine, confidenceEngine, policyEngine, auditLog, new[] { handler });
    }

    [Fact]
    public async Task Process_ThenApprove_RunsFullSuggestPreviewApproveVerifyAuditFlow()
    {
        var pipeline = BuildPipeline();
        var connector = MockConnector.OutlookComposeMissingSubjectAndAttachment();
        var context = await connector.CaptureAsync();

        var suggestions = pipeline.Process(context);
        Assert.Single(suggestions);

        var suggestion = suggestions[0];
        Assert.Equal(SuggestionState.Proposed, suggestion.State);
        Assert.Equal(PolicyAction.Suggest, suggestion.Decision.Action);

        var preview = await pipeline.PreviewAsync(suggestion);
        Assert.False(string.IsNullOrWhiteSpace(preview));
        Assert.Equal(preview, suggestion.PreviewDescription);

        await pipeline.ApproveAsync(suggestion, approvedBy: "test-user");

        Assert.Equal(SuggestionState.Verified, suggestion.State);
        Assert.NotNull(suggestion.VerificationMessage);

        var auditLog = new FileAuditLog(_auditFile);
        var entries = await auditLog.ReadAllAsync();
        Assert.Single(entries);
        Assert.Equal("outlook", entries[0].App);
        Assert.Equal("Missing Email Subject", entries[0].RuleName);
        Assert.Equal("test-user", entries[0].ApprovedBy);
    }

    [Fact]
    public async Task Process_ThenIgnore_DoesNotWriteAuditEntry()
    {
        var pipeline = BuildPipeline();
        var connector = MockConnector.OutlookComposeMissingSubjectAndAttachment();
        var context = await connector.CaptureAsync();

        var suggestion = pipeline.Process(context)[0];
        await pipeline.IgnoreAsync(suggestion);

        Assert.Equal(SuggestionState.Ignored, suggestion.State);

        var auditLog = new FileAuditLog(_auditFile);
        var entries = await auditLog.ReadAllAsync();
        Assert.Empty(entries);
    }

    [Fact]
    public async Task Process_ReturnsNoSuggestions_WhenPolicyCeilingIsObserveOnly()
    {
        var pipeline = BuildPipeline(outlookCeiling: AutonomyLevel.Observe);
        var connector = MockConnector.OutlookComposeMissingSubjectAndAttachment();
        var context = await connector.CaptureAsync();

        var suggestions = pipeline.Process(context);

        Assert.Empty(suggestions);
    }

    public void Dispose()
    {
        if (File.Exists(_auditFile))
        {
            File.Delete(_auditFile);
        }
    }
}
