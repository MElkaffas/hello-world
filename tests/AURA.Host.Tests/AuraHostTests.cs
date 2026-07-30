using Aura.Core.Actions;
using Aura.Core.Connectors;
using Aura.Host.Configuration;
using Aura.Host.Connectors;
using Xunit;

namespace Aura.Host.Tests;

public class AuraHostTests : IDisposable
{
    private readonly List<string> _tempFiles = new();
    private readonly List<string> _tempDirectories = new();

    [Fact]
    public void LoadFromFile_ParsesSampleTenantJson()
    {
        var path = Path.Combine(AppContext.BaseDirectory, "HostSamples", "tenant.default.json");

        var configuration = AuraConfigurationLoader.LoadFromFile(path);

        Assert.Equal("default", configuration.TenantId);
        Assert.Equal(2, configuration.Connectors.Count);
        Assert.Contains(configuration.Connectors, c => c.Implementation == "mock-outlook");
        Assert.Contains(configuration.Connectors, c => c.Implementation == "mock-word");
        Assert.Single(configuration.Policies);
        Assert.Equal("askAndAct", configuration.Policies[0].Ceiling);
    }

    [Fact]
    public void ConnectorRegistry_ResolvesRegisteredImplementation()
    {
        var registry = BuildRegistry();

        var (connector, handler) = registry.Resolve(new ConnectorConfig { Implementation = "mock-outlook" });

        Assert.Equal("outlook", connector.App);
        Assert.NotNull(handler);
    }

    [Fact]
    public void ConnectorRegistry_Throws_ForUnknownImplementation()
    {
        var registry = BuildRegistry();

        Assert.Throws<InvalidOperationException>(() =>
            registry.Resolve(new ConnectorConfig { Implementation = "does-not-exist" }));
    }

    [Fact]
    public async Task Build_ProducesWorkingPipeline_AgainstRegisteredMockOutlookConnector()
    {
        // Isolated rules directory (just the one rule this test cares about) rather than
        // samples/rules/ - that directory's mock Outlook context also matches
        // "Attachment Mentioned But Missing", which would make Assert.Single below flaky
        // against unrelated changes to the sample rule set.
        var rulesDirectory = CreateTempRulesDirectory("""
            {
              "name": "Missing Email Subject",
              "trigger": "email.compose",
              "conditions": [
                { "field": "email.bodyText", "op": "isNotEmpty" },
                { "field": "email.subject", "op": "isEmpty" }
              ],
              "severity": "warning",
              "message": { "en": "Subject is empty" },
              "suggestedAction": { "type": "setField", "field": "email.subject" },
              "autonomy": "suggest",
              "audit": true
            }
            """);

        var configuration = new AuraConfiguration
        {
            TenantId = "test",
            RulesDirectory = rulesDirectory,
            AuditFilePath = TempFile(),
            Policies = new[] { new PolicyRuleConfig { App = "*", Ceiling = "askAndAct" } },
            Connectors = new[] { new ConnectorConfig { Implementation = "mock-outlook", Enabled = true } },
        };

        var host = AuraHost.Build(configuration, BuildRegistry());

        Assert.True(host.Connectors.ContainsKey("outlook"));

        var context = await host.Connectors["outlook"].CaptureAsync();
        var suggestions = host.Pipeline.Process(context);

        Assert.Single(suggestions);

        var suggestion = suggestions[0];

        // The mock connector's body text contains a real-looking email/phone (see
        // MockConnector.OutlookComposeMissingSubjectAndAttachment) - AuraHost.Build should have
        // wired a PrivacyGuard in by default, so the Finding's context is already sanitized.
        Assert.True(suggestion.Finding.Context.SensitiveRedacted);
        Assert.DoesNotContain("jane.doe@example.com", suggestion.Finding.Context.Email!.BodyText);

        await host.Pipeline.PreviewAsync(suggestion);
        await host.Pipeline.ApproveAsync(suggestion, approvedBy: "test-user");

        Assert.Equal(SuggestionState.Verified, suggestion.State);

        var entries = await host.AuditSink.ReadAllAsync();
        Assert.Single(entries);
        Assert.Equal("test-user", entries[0].ApprovedBy);
    }

    [Fact]
    public async Task Build_AppliesConfiguredPrivacyZone_ClearingTheZonedField()
    {
        var rulesDirectory = CreateTempRulesDirectory("""
            {
              "name": "Missing Email Subject",
              "trigger": "email.compose",
              "conditions": [
                { "field": "email.bodyText", "op": "isNotEmpty" },
                { "field": "email.subject", "op": "isEmpty" }
              ],
              "autonomy": "suggest"
            }
            """);

        var configuration = new AuraConfiguration
        {
            RulesDirectory = rulesDirectory,
            AuditFilePath = TempFile(),
            Connectors = new[] { new ConnectorConfig { Implementation = "mock-outlook", Enabled = true } },
            PrivacyZones = new[]
            {
                new PrivacyZoneConfig { Name = "No body capture", App = "outlook", Field = "email.bodyText" },
            },
        };

        var host = AuraHost.Build(configuration, BuildRegistry());
        var context = await host.Connectors["outlook"].CaptureAsync();
        var suggestions = host.Pipeline.Process(context);

        // The zoned field is wholesale replaced, so "isNotEmpty" on it still passes (it's not
        // empty, just not real content) and the rule still fires - only the field's value changed.
        Assert.Single(suggestions);
        Assert.Equal("[PRIVACY ZONE]", suggestions[0].Finding.Context.Email!.BodyText);
    }

    [Fact]
    public void Build_SkipsDisabledConnectors()
    {
        var configuration = new AuraConfiguration
        {
            RulesDirectory = Path.Combine(AppContext.BaseDirectory, "Rules"),
            AuditFilePath = TempFile(),
            Connectors = new[]
            {
                new ConnectorConfig { Implementation = "mock-outlook", Enabled = false },
                new ConnectorConfig { Implementation = "mock-word", Enabled = true },
            },
        };

        var host = AuraHost.Build(configuration, BuildRegistry());

        Assert.False(host.Connectors.ContainsKey("outlook"));
        Assert.True(host.Connectors.ContainsKey("word"));
    }

    private static ConnectorRegistry BuildRegistry() => new(new IConnectorFactory[]
    {
        new MockOutlookConnectorFactory(),
        new MockWordConnectorFactory(),
    });

    private string TempFile()
    {
        var path = Path.Combine(Path.GetTempPath(), $"aura-host-test-{Guid.NewGuid():N}.jsonl");
        _tempFiles.Add(path);
        return path;
    }

    private string CreateTempRulesDirectory(string ruleJson)
    {
        var directory = Path.Combine(Path.GetTempPath(), $"aura-host-test-rules-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        File.WriteAllText(Path.Combine(directory, "rule.json"), ruleJson);
        _tempDirectories.Add(directory);
        return directory;
    }

    public void Dispose()
    {
        foreach (var file in _tempFiles.Where(File.Exists))
        {
            File.Delete(file);
        }

        foreach (var directory in _tempDirectories.Where(Directory.Exists))
        {
            Directory.Delete(directory, recursive: true);
        }
    }
}
