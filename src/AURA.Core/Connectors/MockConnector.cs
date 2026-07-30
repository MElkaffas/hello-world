using Aura.Core.Models;
using Aura.Core.Profiles;

namespace Aura.Core.Connectors;

/// <summary>
/// Stands in for a real connector while wiring the Context → Rule → Policy → Notification
/// pipeline end to end, before Graph API integration lands (per the build order in the brief).
/// </summary>
public sealed class MockConnector : IConnector
{
    private readonly Func<AuraContext> _factory;

    public MockConnector(string app, Func<AuraContext> factory)
    {
        App = app;
        _factory = factory;
    }

    public string App { get; }

    public Task<AuraContext> CaptureAsync(CancellationToken cancellationToken = default) =>
        Task.FromResult(_factory());

    /// <summary>Outlook compose window: body mentions an attachment but none is attached, subject
    /// empty, and the body contains real-looking PII (an email address and a phone number) so
    /// the Privacy Guard's redaction has something genuine to catch on every capture.</summary>
    public static MockConnector OutlookComposeMissingSubjectAndAttachment() => new("outlook", () => new AuraContext
    {
        App = "outlook",
        Window = "Compose",
        Scope = "region",
        Trigger = "email.compose",
        Email = new EmailContext
        {
            To = new[] { "client@example.com" },
            Cc = Array.Empty<string>(),
            Subject = string.Empty,
            BodyText = "Please find the report attached for your review. " +
                       "Contact me directly at jane.doe@example.com or 01012345678 if you have questions.",
            MentionsAttachment = true,
            Attachments = Array.Empty<string>(),
        },
    });

    /// <summary>Word document whose title/body fonts do not match the active Document Standard Profile.</summary>
    public static MockConnector WordDocumentOffProfile() => new("word", () =>
    {
        var profile = new DocumentStandardProfile
        {
            Name = "Corporate Memo",
            Roles = new Dictionary<string, StyleRequirement>
            {
                ["title"] = new() { Font = "Arial", Size = 16, Bold = true },
                ["body"] = new() { Font = "Arial", Size = 12, Bold = false },
            },
        };

        var document = new DocumentContext
        {
            ProfileName = profile.Name,
            Styles = new[]
            {
                new DocumentStyleRun { Role = "title", Font = "Times New Roman", Size = 22, Bold = true },
                new DocumentStyleRun { Role = "body", Font = "Calibri", Size = 11, Bold = false },
            },
        };

        var compliance = DocumentProfileChecker.Check(document, profile);

        return new AuraContext
        {
            App = "word",
            Window = "Document1.docx",
            Scope = "document",
            Trigger = "document.edit",
            Document = new DocumentContext
            {
                ProfileName = document.ProfileName,
                Styles = document.Styles,
                ComplianceScore = compliance.Score,
                ComplianceIssues = compliance.Issues,
            },
        };
    });
}
