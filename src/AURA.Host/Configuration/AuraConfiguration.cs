namespace Aura.Host.Configuration;

/// <summary>
/// Single-tenant, file-based configuration for one running Desktop Agent instance. This is
/// deliberately NOT a multi-tenant model - per the architecture review, multi-tenancy belongs
/// in a future server/backend, not inside the Desktop Agent. What the Agent needs to know about
/// itself (TenantId, rules, policies, which connectors are enabled) lives here instead of being
/// hardcoded in <c>AssistantController</c>.
/// </summary>
public sealed class AuraConfiguration
{
    public string TenantId { get; init; } = "default";

    /// <summary>Directory of JSON rule files (README §11), relative to the app's base directory
    /// unless rooted.</summary>
    public string RulesDirectory { get; init; } = "Rules";

    /// <summary>Which registered <see cref="Connectors.IConnectorFactory"/> implementation backs
    /// the audit sink. "local-file" is the only one implemented today.</summary>
    public string AuditSink { get; init; } = "local-file";

    /// <summary>Supports the <c>{localAppData}</c> token, expanded by <see cref="AuraPaths"/>.</summary>
    public string AuditFilePath { get; init; } = "{localAppData}/AURA/audit.jsonl";

    public IReadOnlyList<PolicyRuleConfig> Policies { get; init; } = Array.Empty<PolicyRuleConfig>();
    public IReadOnlyList<ConnectorConfig> Connectors { get; init; } = Array.Empty<ConnectorConfig>();

    /// <summary>Fields that must never leave this process in readable form (README §10),
    /// evaluated before PII detection/redaction on every capture.</summary>
    public IReadOnlyList<PrivacyZoneConfig> PrivacyZones { get; init; } = Array.Empty<PrivacyZoneConfig>();

    /// <summary>Session-based permission (README §10): monitoring auto-stops after this many
    /// minutes. 0 or negative disables the limit.</summary>
    public double SessionTimeLimitMinutes { get; init; } = 30;
}

/// <summary>Maps to <see cref="Aura.Core.Security.PrivacyZone"/>.</summary>
public sealed class PrivacyZoneConfig
{
    public required string Name { get; init; }
    public string App { get; init; } = "*";
    public required string Field { get; init; }
}

/// <summary>Maps to <see cref="Aura.Core.Policy.PolicyRule"/> - kept as plain strings here so the
/// JSON file stays human-editable without referencing Core's enum type directly.</summary>
public sealed class PolicyRuleConfig
{
    public string App { get; init; } = "*";
    public string? RuleName { get; init; }

    /// <summary>observe | notify | suggest | askAndAct | autoAct</summary>
    public string Ceiling { get; init; } = "suggest";
}

/// <summary>One enabled connector. <see cref="Implementation"/> is the registry key (e.g.
/// "mock-outlook") - independent from the app id the produced <c>IConnector.App</c> reports,
/// so swapping which implementation backs "outlook" never touches the Rule Engine.</summary>
public sealed class ConnectorConfig
{
    public required string Implementation { get; init; }
    public bool Enabled { get; init; } = true;
    public IReadOnlyDictionary<string, string> Settings { get; init; } = new Dictionary<string, string>();
}
