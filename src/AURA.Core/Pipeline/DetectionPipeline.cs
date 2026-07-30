using Aura.Core.Actions;
using Aura.Core.Audit;
using Aura.Core.Confidence;
using Aura.Core.Models;
using Aura.Core.Policy;
using Aura.Core.Rules;

namespace Aura.Core.Pipeline;

/// <summary>
/// Wires the full detection pipeline from README §6:
/// Context (from a connector) → Rule Engine → Confidence Engine → Policy Engine → Suggestion,
/// then Preview → Approve → Verify → Audit once the user acts (README §7).
/// </summary>
public sealed class DetectionPipeline
{
    private readonly IRuleEngine _ruleEngine;
    private readonly IConfidenceEngine _confidenceEngine;
    private readonly IPolicyEngine _policyEngine;
    private readonly IAuditLog _auditLog;
    private readonly Dictionary<string, IActionHandler> _handlers;

    public DetectionPipeline(
        IRuleEngine ruleEngine,
        IConfidenceEngine confidenceEngine,
        IPolicyEngine policyEngine,
        IAuditLog auditLog,
        IEnumerable<IActionHandler> handlers)
    {
        _ruleEngine = ruleEngine;
        _confidenceEngine = confidenceEngine;
        _policyEngine = policyEngine;
        _auditLog = auditLog;
        _handlers = handlers.ToDictionary(h => h.App, StringComparer.OrdinalIgnoreCase);
    }

    /// <summary>Runs Rule → Confidence → Policy over one captured context. Findings the policy
    /// suppresses or merely observes are not returned - only ones worth surfacing to the user.</summary>
    public IReadOnlyList<Suggestion> Process(AuraContext context)
    {
        var findings = _ruleEngine.Evaluate(context);
        var suggestions = new List<Suggestion>();

        foreach (var finding in findings)
        {
            finding.Confidence = _confidenceEngine.Score(finding);
            var band = _confidenceEngine.Band(finding.Confidence);
            var decision = _policyEngine.Decide(finding, band);

            if (decision.Action is PolicyAction.Suppress or PolicyAction.Observe)
            {
                continue;
            }

            suggestions.Add(new Suggestion { Finding = finding, Decision = decision });
        }

        return suggestions;
    }

    public async Task<string> PreviewAsync(Suggestion suggestion, CancellationToken cancellationToken = default)
    {
        var handler = ResolveHandler(suggestion);
        var preview = await handler.PreviewAsync(suggestion, cancellationToken);
        suggestion.PreviewDescription = preview;
        return preview;
    }

    /// <summary>Applies, verifies, and audits a suggestion. Call only after explicit user
    /// approval, unless the policy decision already authorizes Auto-Act.</summary>
    public async Task ApproveAsync(Suggestion suggestion, string approvedBy, CancellationToken cancellationToken = default)
    {
        var handler = ResolveHandler(suggestion);

        suggestion.State = SuggestionState.Approved;
        await handler.ApplyAsync(suggestion, cancellationToken);
        suggestion.State = SuggestionState.Applied;

        var verified = await handler.VerifyAsync(suggestion, cancellationToken);
        suggestion.State = verified ? SuggestionState.Verified : SuggestionState.Failed;

        if (suggestion.Finding.Rule.Audit)
        {
            await _auditLog.RecordAsync(new AuditEntry
            {
                App = suggestion.Finding.Context.App,
                RuleName = suggestion.Finding.Rule.Name,
                Action = suggestion.Finding.Rule.SuggestedAction?.Type ?? "manual-review",
                Before = suggestion.PreviewDescription,
                After = suggestion.VerificationMessage,
                ApprovedBy = approvedBy,
            }, cancellationToken);
        }
    }

    public Task IgnoreAsync(Suggestion suggestion, CancellationToken cancellationToken = default)
    {
        suggestion.State = SuggestionState.Ignored;
        return Task.CompletedTask;
    }

    private IActionHandler ResolveHandler(Suggestion suggestion)
    {
        if (_handlers.TryGetValue(suggestion.Finding.Context.App, out var handler))
        {
            return handler;
        }

        throw new InvalidOperationException(
            $"No action handler registered for app '{suggestion.Finding.Context.App}'.");
    }
}
