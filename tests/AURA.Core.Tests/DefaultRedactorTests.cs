using Aura.Core.Security;
using Xunit;

namespace Aura.Core.Tests;

public class DefaultRedactorTests
{
    private readonly DefaultRedactor _redactor = new();

    [Fact]
    public void Redact_ReplacesSingleMatch()
    {
        const string text = "Email me at jane@example.com please.";
        var matches = new[]
        {
            new SensitiveDataMatch { Category = "email", Value = "jane@example.com", Index = 12, Length = 16 },
        };

        var result = _redactor.Redact(text, matches);

        Assert.Equal("Email me at [REDACTED] please.", result);
    }

    [Fact]
    public void Redact_MergesOverlappingMatches_WithoutCorruptingText()
    {
        // Simulates two detectors both matching the same digit run under different categories
        // (e.g. "nationalId" and "creditCard" both matching a 14-digit number).
        const string text = "ID 29801011234567 end";
        var matches = new[]
        {
            new SensitiveDataMatch { Category = "nationalId", Value = "29801011234567", Index = 3, Length = 14 },
            new SensitiveDataMatch { Category = "creditCard", Value = "29801011234567", Index = 3, Length = 14 },
        };

        var result = _redactor.Redact(text, matches);

        Assert.Equal("ID [REDACTED] end", result);
    }

    [Fact]
    public void Redact_HandlesMultipleNonOverlappingMatches_InOriginalOrder()
    {
        const string text = "a@b.com and c@d.com";
        var matches = new[]
        {
            new SensitiveDataMatch { Category = "email", Value = "a@b.com", Index = 0, Length = 7 },
            new SensitiveDataMatch { Category = "email", Value = "c@d.com", Index = 12, Length = 7 },
        };

        var result = _redactor.Redact(text, matches);

        Assert.Equal("[REDACTED] and [REDACTED]", result);
    }

    [Fact]
    public void Redact_ReturnsOriginalText_WhenNoMatches()
    {
        const string text = "nothing sensitive here";

        var result = _redactor.Redact(text, Array.Empty<SensitiveDataMatch>());

        Assert.Equal(text, result);
    }
}
