using Aura.Core.Models;
using Aura.Core.Security;
using Xunit;

namespace Aura.Core.Tests;

public class PrivacyGuardTests
{
    private static AuraContext OutlookContext(string bodyText, string subject = "") => new()
    {
        App = "outlook",
        Trigger = "email.compose",
        Email = new EmailContext
        {
            To = new[] { "client@example.com" },
            Subject = subject,
            BodyText = bodyText,
            MentionsAttachment = false,
        },
    };

    [Fact]
    public void Apply_RedactsDetectedPii_InBodyText()
    {
        var guard = new PrivacyGuard(Array.Empty<PrivacyZone>(), new RegexSensitiveDataDetector(), new DefaultRedactor());
        var context = OutlookContext("Reach me at jane.doe@example.com anytime.");

        var sanitized = guard.Apply(context);

        Assert.DoesNotContain("jane.doe@example.com", sanitized.Email!.BodyText);
        Assert.Contains("[REDACTED]", sanitized.Email!.BodyText);
    }

    [Fact]
    public void Apply_MarksContextAsRedacted()
    {
        var guard = new PrivacyGuard(Array.Empty<PrivacyZone>(), new RegexSensitiveDataDetector(), new DefaultRedactor());
        var context = OutlookContext("Nothing sensitive here.");

        var sanitized = guard.Apply(context);

        Assert.True(sanitized.SensitiveRedacted);
        Assert.True(sanitized.Security?.RedactionApplied);
    }

    [Fact]
    public void Apply_LeavesTextUnchanged_WhenNoPiiDetected()
    {
        var guard = new PrivacyGuard(Array.Empty<PrivacyZone>(), new RegexSensitiveDataDetector(), new DefaultRedactor());
        var context = OutlookContext("Please find the report attached for your review.");

        var sanitized = guard.Apply(context);

        Assert.Equal("Please find the report attached for your review.", sanitized.Email!.BodyText);
    }

    [Fact]
    public void Apply_ClearsWholeField_WhenItMatchesAPrivacyZone()
    {
        var zones = new[] { new PrivacyZone { Name = "No body capture", App = "outlook", Field = "email.bodyText" } };
        var guard = new PrivacyGuard(zones, new RegexSensitiveDataDetector(), new DefaultRedactor());
        var context = OutlookContext("This text should never be visible.");

        var sanitized = guard.Apply(context);

        Assert.Equal("[PRIVACY ZONE]", sanitized.Email!.BodyText);
    }

    [Fact]
    public void Apply_DoesNotClearField_WhenZoneIsForADifferentApp()
    {
        var zones = new[] { new PrivacyZone { Name = "Word only", App = "word", Field = "email.bodyText" } };
        var guard = new PrivacyGuard(zones, new RegexSensitiveDataDetector(), new DefaultRedactor());
        var context = OutlookContext("Please find the report attached for your review.");

        var sanitized = guard.Apply(context);

        Assert.Equal("Please find the report attached for your review.", sanitized.Email!.BodyText);
    }

    [Fact]
    public void Apply_PreservesNonSensitiveFields()
    {
        var guard = new PrivacyGuard(Array.Empty<PrivacyZone>(), new RegexSensitiveDataDetector(), new DefaultRedactor());
        var context = OutlookContext("Nothing sensitive here.", subject: "Quarterly report");

        var sanitized = guard.Apply(context);

        Assert.Equal("outlook", sanitized.App);
        Assert.Equal("email.compose", sanitized.Trigger);
        Assert.Equal("Quarterly report", sanitized.Email!.Subject);
        Assert.Equal(context.Email!.To, sanitized.Email!.To);
    }
}
