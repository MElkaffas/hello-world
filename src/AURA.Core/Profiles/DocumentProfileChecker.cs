using Aura.Core.Models;

namespace Aura.Core.Profiles;

/// <summary>Compares a document's actual styles to its active <see cref="DocumentStandardProfile"/>.</summary>
public static class DocumentProfileChecker
{
    public static DocumentComplianceResult Check(DocumentContext document, DocumentStandardProfile profile)
    {
        var issues = new List<string>();
        var checkedRoles = 0;

        foreach (var run in document.Styles)
        {
            if (!profile.Roles.TryGetValue(run.Role, out var expected))
            {
                continue;
            }

            checkedRoles++;

            if (!string.Equals(run.Font, expected.Font, StringComparison.OrdinalIgnoreCase))
            {
                issues.Add($"{run.Role}: expected font '{expected.Font}', found '{run.Font}'.");
            }

            if (Math.Abs(run.Size - expected.Size) > 0.01)
            {
                issues.Add($"{run.Role}: expected size {expected.Size}, found {run.Size}.");
            }

            if (run.Bold != expected.Bold)
            {
                issues.Add($"{run.Role}: expected bold={expected.Bold}, found {run.Bold}.");
            }

            if (profile.Alignment is not null &&
                run.Alignment is not null &&
                !string.Equals(run.Alignment, profile.Alignment, StringComparison.OrdinalIgnoreCase))
            {
                issues.Add($"{run.Role}: expected alignment '{profile.Alignment}', found '{run.Alignment}'.");
            }
        }

        if (checkedRoles == 0)
        {
            return new DocumentComplianceResult { Score = 1.0, Issues = Array.Empty<string>() };
        }

        var score = Math.Max(0.0, 1.0 - (double)issues.Count / (checkedRoles * 3));
        return new DocumentComplianceResult { Score = score, Issues = issues };
    }
}
