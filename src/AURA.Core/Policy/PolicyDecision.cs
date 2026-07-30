using Aura.Core.Confidence;
using Aura.Core.Models;

namespace Aura.Core.Policy;

public sealed class PolicyDecision
{
    public required Finding Finding { get; init; }
    public required AutonomyLevel EffectiveLevel { get; init; }
    public required ConfidenceBand ConfidenceBand { get; init; }
    public required PolicyAction Action { get; init; }
}
