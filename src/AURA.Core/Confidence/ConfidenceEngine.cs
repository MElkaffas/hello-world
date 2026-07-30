using Aura.Core.Models;

namespace Aura.Core.Confidence;

/// <summary>
/// Scores findings so the Policy Engine can gate interruptions (README §9):
/// &gt;90% alert, 70-90% suggest quietly, &lt;70% do not interrupt. Purpose: minimize
/// false positives and "AI nagging".
/// </summary>
public sealed class ConfidenceEngine : IConfidenceEngine
{
    private readonly double _alertThreshold;
    private readonly double _suggestThreshold;

    private static readonly HashSet<string> DeterministicOperators = new(StringComparer.OrdinalIgnoreCase)
    {
        "isEmpty", "isNotEmpty", "equals", "notEquals", "in",
    };

    public ConfidenceEngine(double alertThreshold = 0.90, double suggestThreshold = 0.70)
    {
        if (suggestThreshold > alertThreshold)
        {
            throw new ArgumentException("suggestThreshold must be <= alertThreshold.");
        }

        _alertThreshold = alertThreshold;
        _suggestThreshold = suggestThreshold;
    }

    public double Score(Finding finding)
    {
        if (finding.Rule.BaseConfidence is double baseConfidence)
        {
            return Math.Clamp(baseConfidence, 0.0, 1.0);
        }

        // Findings built entirely from deterministic, structural conditions (a field is empty,
        // equals a literal, etc.) are trustworthy. Anything relying on fuzzier operators
        // (contains/regex/comparisons - closer to heuristic text matching) gets a lower default
        // so it lands in "suggest quietly" rather than an interrupting alert.
        var allDeterministic = finding.Rule.Conditions.Count > 0
            && finding.Rule.Conditions.All(c => DeterministicOperators.Contains(c.Op));

        return allDeterministic ? 0.95 : 0.75;
    }

    public ConfidenceBand Band(double score)
    {
        if (score > _alertThreshold) return ConfidenceBand.Alert;
        if (score >= _suggestThreshold) return ConfidenceBand.Suggest;
        return ConfidenceBand.Suppress;
    }
}
