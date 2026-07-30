using Aura.Core.Confidence;
using Aura.Core.Models;

namespace Aura.Core.Policy;

public interface IPolicyEngine
{
    void Configure(IEnumerable<PolicyRule> policyRules);

    /// <summary>Decides observe/notify/suggest/ask&amp;act/auto-act from the rule's requested
    /// autonomy, the admin's configured ceiling, and the finding's confidence band (README §8).</summary>
    PolicyDecision Decide(Finding finding, ConfidenceBand band);
}
