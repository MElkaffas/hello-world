using System.Text;

namespace Aura.Core.Security;

/// <summary>Replaces each detected span with a fixed marker. Overlapping/adjacent matches
/// (e.g. a national-ID pattern nested inside a broader digit-run pattern) are merged into a
/// single span first, so overlapping detectors never corrupt the text or double-redact.</summary>
public sealed class DefaultRedactor : IRedactor
{
    private const string Marker = "[REDACTED]";

    public string Redact(string text, IReadOnlyList<SensitiveDataMatch> matches)
    {
        if (string.IsNullOrEmpty(text) || matches.Count == 0)
        {
            return text;
        }

        var spans = MergeOverlapping(matches);
        var builder = new StringBuilder(text);

        foreach (var span in spans.OrderByDescending(s => s.Start))
        {
            builder.Remove(span.Start, span.Length);
            builder.Insert(span.Start, Marker);
        }

        return builder.ToString();
    }

    private static List<(int Start, int Length)> MergeOverlapping(IReadOnlyList<SensitiveDataMatch> matches)
    {
        var ordered = matches.OrderBy(m => m.Index).ToList();
        var merged = new List<(int Start, int Length)>();

        foreach (var match in ordered)
        {
            var start = match.Index;
            var end = match.Index + match.Length;

            if (merged.Count > 0 && start <= merged[^1].Start + merged[^1].Length)
            {
                var last = merged[^1];
                var newEnd = Math.Max(last.Start + last.Length, end);
                merged[^1] = (last.Start, newEnd - last.Start);
            }
            else
            {
                merged.Add((start, end - start));
            }
        }

        return merged;
    }
}
