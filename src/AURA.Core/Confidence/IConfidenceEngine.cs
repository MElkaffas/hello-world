using Aura.Core.Models;

namespace Aura.Core.Confidence;

public enum ConfidenceBand
{
    /// <summary>&lt; 70% - do not interrupt the user (README §9).</summary>
    Suppress,

    /// <summary>70-90% - suggest quietly.</summary>
    Suggest,

    /// <summary>&gt; 90% - alert.</summary>
    Alert,
}

public interface IConfidenceEngine
{
    /// <summary>Returns a 0-1 confidence score for a finding.</summary>
    double Score(Finding finding);

    ConfidenceBand Band(double score);
}
