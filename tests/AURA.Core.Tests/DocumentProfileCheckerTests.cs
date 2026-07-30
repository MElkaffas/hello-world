using System.Text.Json;
using Aura.Core;
using Aura.Core.Models;
using Aura.Core.Profiles;
using Xunit;

namespace Aura.Core.Tests;

public class DocumentProfileCheckerTests
{
    private static DocumentStandardProfile CorporateMemo() => new()
    {
        Name = "Corporate Memo",
        Roles = new Dictionary<string, StyleRequirement>
        {
            ["title"] = new() { Font = "Arial", Size = 16, Bold = true },
            ["body"] = new() { Font = "Arial", Size = 12, Bold = false },
        },
        Alignment = "Justified",
    };

    [Fact]
    public void Check_ReturnsFullScoreAndNoIssues_WhenDocumentMatchesProfile()
    {
        var document = new DocumentContext
        {
            Styles = new[]
            {
                new DocumentStyleRun { Role = "title", Font = "Arial", Size = 16, Bold = true, Alignment = "Justified" },
                new DocumentStyleRun { Role = "body", Font = "Arial", Size = 12, Bold = false, Alignment = "Justified" },
            },
        };

        var result = DocumentProfileChecker.Check(document, CorporateMemo());

        Assert.Equal(1.0, result.Score);
        Assert.Empty(result.Issues);
    }

    [Fact]
    public void Check_ReportsFontAndSizeMismatches()
    {
        var document = new DocumentContext
        {
            Styles = new[]
            {
                new DocumentStyleRun { Role = "title", Font = "Times New Roman", Size = 22, Bold = true },
            },
        };

        var result = DocumentProfileChecker.Check(document, CorporateMemo());

        Assert.True(result.Score < 1.0);
        Assert.Contains(result.Issues, i => i.Contains("font"));
        Assert.Contains(result.Issues, i => i.Contains("size"));
    }

    [Fact]
    public void Check_IgnoresRolesNotDefinedInProfile()
    {
        var document = new DocumentContext
        {
            Styles = new[]
            {
                new DocumentStyleRun { Role = "footer", Font = "Comic Sans MS", Size = 8, Bold = true },
            },
        };

        var result = DocumentProfileChecker.Check(document, CorporateMemo());

        Assert.Equal(1.0, result.Score);
        Assert.Empty(result.Issues);
    }

    [Fact]
    public void SampleProfileJson_DeserializesIntoDocumentStandardProfile()
    {
        var path = Path.Combine(AppContext.BaseDirectory, "Profiles", "corporate-memo.json");

        var json = File.ReadAllText(path);
        var profile = JsonSerializer.Deserialize<DocumentStandardProfile>(json, AuraJson.Options);

        Assert.NotNull(profile);
        Assert.Equal("Corporate Memo", profile!.Name);
        Assert.Equal("Arial", profile.Roles["title"].Font);
        Assert.Equal(16, profile.Roles["title"].Size);
        Assert.Equal("Justified", profile.Alignment);
    }
}
