using Aura.Core.Actions;
using Aura.Core.Audit;
using Aura.Core.Confidence;
using Aura.Core.Connectors;
using Aura.Core.Pipeline;
using Aura.Core.Policy;
using Aura.Core.Rules;
using Aura.Core.Security;
using Aura.Host.Configuration;
using Aura.Host.Connectors;
using Microsoft.Extensions.DependencyInjection;

namespace Aura.Host;

/// <summary>
/// Composition root: turns an <see cref="AuraConfiguration"/> + <see cref="ConnectorRegistry"/>
/// into a ready-to-use <see cref="DetectionPipeline"/> and resolved connector set. This is the
/// code that used to live inside <c>AssistantController</c>'s constructor - moved here so the
/// Desktop project only owns UI, and so a future non-WPF host (a service, a test harness) can
/// reuse the exact same wiring.
/// </summary>
public sealed class AuraHost
{
    public AuraConfiguration Configuration { get; }
    public DetectionPipeline Pipeline { get; }
    public IReadOnlyDictionary<string, IConnector> Connectors { get; }
    public IAuditSink AuditSink { get; }

    private AuraHost(
        AuraConfiguration configuration,
        DetectionPipeline pipeline,
        IReadOnlyDictionary<string, IConnector> connectors,
        IAuditSink auditSink)
    {
        Configuration = configuration;
        Pipeline = pipeline;
        Connectors = connectors;
        AuditSink = auditSink;
    }

    public static AuraHost Build(AuraConfiguration configuration, ConnectorRegistry registry)
    {
        var connectors = new Dictionary<string, IConnector>(StringComparer.OrdinalIgnoreCase);
        var handlers = new List<IActionHandler>();

        foreach (var connectorConfig in configuration.Connectors.Where(c => c.Enabled))
        {
            var (connector, handler) = registry.Resolve(connectorConfig);
            connectors[connector.App] = connector;
            if (handler is not null)
            {
                handlers.Add(handler);
            }
        }

        var services = new ServiceCollection();

        services.AddSingleton(configuration);

        services.AddSingleton<IRuleEngine>(_ =>
        {
            var engine = new RuleEngine();
            var rulesDirectory = AuraPaths.ResolveDirectory(configuration.RulesDirectory);
            if (Directory.Exists(rulesDirectory))
            {
                engine.LoadRulesFromDirectory(rulesDirectory);
            }

            return engine;
        });

        services.AddSingleton<IConfidenceEngine, ConfidenceEngine>();

        services.AddSingleton<IPolicyEngine>(_ =>
        {
            var engine = new PolicyEngine();
            engine.Configure(configuration.Policies.Select(ToPolicyRule));
            return engine;
        });

        services.AddSingleton<IAuditSink>(_ => new LocalFileAuditSink(AuraPaths.Expand(configuration.AuditFilePath)));

        services.AddSingleton<IPrivacyGuard>(_ => new PrivacyGuard(
            configuration.PrivacyZones.Select(ToPrivacyZone).ToList(),
            new RegexSensitiveDataDetector(),
            new DefaultRedactor()));

        services.AddSingleton(sp => new DetectionPipeline(
            sp.GetRequiredService<IRuleEngine>(),
            sp.GetRequiredService<IConfidenceEngine>(),
            sp.GetRequiredService<IPolicyEngine>(),
            sp.GetRequiredService<IAuditSink>(),
            handlers,
            sp.GetRequiredService<IPrivacyGuard>()));

        var provider = services.BuildServiceProvider();
        var pipeline = provider.GetRequiredService<DetectionPipeline>();
        var auditSink = provider.GetRequiredService<IAuditSink>();

        return new AuraHost(configuration, pipeline, connectors, auditSink);
    }

    private static PolicyRule ToPolicyRule(PolicyRuleConfig config) => new()
    {
        App = config.App,
        RuleName = config.RuleName,
        Ceiling = ParseAutonomy(config.Ceiling),
    };

    private static PrivacyZone ToPrivacyZone(PrivacyZoneConfig config) => new()
    {
        Name = config.Name,
        App = config.App,
        Field = config.Field,
    };

    private static AutonomyLevel ParseAutonomy(string ceiling) => ceiling.ToLowerInvariant() switch
    {
        "observe" => AutonomyLevel.Observe,
        "notify" => AutonomyLevel.Notify,
        "suggest" => AutonomyLevel.Suggest,
        "askandact" or "ask&act" or "askact" => AutonomyLevel.AskAndAct,
        "autoact" or "auto-act" => AutonomyLevel.AutoAct,
        _ => AutonomyLevel.Suggest,
    };
}
