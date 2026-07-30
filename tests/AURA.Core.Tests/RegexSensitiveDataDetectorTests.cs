using Aura.Core.Security;
using Xunit;

namespace Aura.Core.Tests;

public class RegexSensitiveDataDetectorTests
{
    private readonly RegexSensitiveDataDetector _detector = new();

    [Fact]
    public void Detect_FindsEmailAddress()
    {
        var matches = _detector.Detect("Contact jane.doe@example.com for details.");

        Assert.Contains(matches, m => m.Category == "email" && m.Value == "jane.doe@example.com");
    }

    [Fact]
    public void Detect_FindsEgyptianMobileNumber()
    {
        var matches = _detector.Detect("Call me on 01012345678 tomorrow.");

        Assert.Contains(matches, m => m.Category == "phone" && m.Value == "01012345678");
    }

    [Fact]
    public void Detect_FindsFourteenDigitNationalId()
    {
        var matches = _detector.Detect("National ID: 29801011234567.");

        Assert.Contains(matches, m => m.Category == "nationalId");
    }

    [Fact]
    public void Detect_ReturnsEmpty_ForPlainText()
    {
        var matches = _detector.Detect("Please find the report attached for your review.");

        Assert.Empty(matches);
    }

    [Fact]
    public void Detect_ReturnsEmpty_ForNullOrEmptyText()
    {
        Assert.Empty(_detector.Detect(string.Empty));
        Assert.Empty(_detector.Detect(null!));
    }
}
