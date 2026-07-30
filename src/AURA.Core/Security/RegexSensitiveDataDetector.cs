using System.Text.RegularExpressions;

namespace Aura.Core.Security;

/// <summary>
/// Deterministic, offline PII detection via regex - no AI call, no network, safe to run
/// regardless of any local-vs-cloud routing decision. Intentionally narrow (a handful of
/// well-known patterns) rather than a false claim of comprehensive PII coverage - treat this as
/// a floor, not the final word. A broader data-classification pass is a later concern.
/// </summary>
public sealed class RegexSensitiveDataDetector : ISensitiveDataDetector
{
    private static readonly (string Category, Regex Pattern)[] Patterns =
    {
        ("email", new Regex(@"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}", RegexOptions.Compiled)),
        ("nationalId", new Regex(@"\b\d{14}\b", RegexOptions.Compiled)),
        ("creditCard", new Regex(@"\b(?:\d[ -]?){13,19}\b", RegexOptions.Compiled)),
        ("phone", new Regex(@"\b(?:\+?\d{1,3}[ -]?)?01\d{9}\b", RegexOptions.Compiled)),
    };

    public IReadOnlyList<SensitiveDataMatch> Detect(string text)
    {
        if (string.IsNullOrEmpty(text))
        {
            return Array.Empty<SensitiveDataMatch>();
        }

        var matches = new List<SensitiveDataMatch>();

        foreach (var (category, pattern) in Patterns)
        {
            foreach (Match match in pattern.Matches(text))
            {
                matches.Add(new SensitiveDataMatch
                {
                    Category = category,
                    Value = match.Value,
                    Index = match.Index,
                    Length = match.Length,
                });
            }
        }

        return matches.OrderBy(m => m.Index).ToList();
    }
}
