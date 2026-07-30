using System.Text.Json;
using Aura.Core;

namespace Aura.Host.Configuration;

public static class AuraConfigurationLoader
{
    public static AuraConfiguration LoadFromFile(string path)
    {
        var json = File.ReadAllText(path);
        return JsonSerializer.Deserialize<AuraConfiguration>(json, AuraJson.Options)
            ?? throw new InvalidOperationException($"'{path}' deserialized to null.");
    }

    /// <summary>The bundled single-tenant default (mock connectors only, Suggest-capped policy) -
    /// used when no external configuration file is supplied, e.g. first run or tests.</summary>
    public static AuraConfiguration Default() => new()
    {
        TenantId = "default",
        RulesDirectory = "Rules",
        AuditSink = "local-file",
        AuditFilePath = "{localAppData}/AURA/audit.jsonl",
        Policies = new[] { new PolicyRuleConfig { App = "*", Ceiling = "askAndAct" } },
        Connectors = new[]
        {
            new ConnectorConfig { Implementation = "mock-outlook", Enabled = true },
            new ConnectorConfig { Implementation = "mock-word", Enabled = true },
        },
    };
}
