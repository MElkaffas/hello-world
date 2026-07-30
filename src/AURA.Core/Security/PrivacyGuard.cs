using Aura.Core.Models;

namespace Aura.Core.Security;

/// <summary>
/// Default <see cref="IPrivacyGuard"/>: clears any field matching a configured
/// <see cref="PrivacyZone"/>, then runs PII detection + redaction on the free-text fields left.
/// Only Email's Subject/BodyText are scanned today - Document has no comparable free-text field
/// yet (its styles are font/size metadata, not user-authored text).
/// </summary>
public sealed class PrivacyGuard : IPrivacyGuard
{
    private const string ZoneRedactionText = "[PRIVACY ZONE]";

    private readonly IReadOnlyList<PrivacyZone> _zones;
    private readonly ISensitiveDataDetector _detector;
    private readonly IRedactor _redactor;

    public PrivacyGuard(IReadOnlyList<PrivacyZone> zones, ISensitiveDataDetector detector, IRedactor redactor)
    {
        _zones = zones;
        _detector = detector;
        _redactor = redactor;
    }

    public AuraContext Apply(AuraContext context)
    {
        var email = context.Email;

        if (email is not null)
        {
            email = new EmailContext
            {
                To = email.To,
                Cc = email.Cc,
                Subject = Sanitize(context.App, "email.subject", email.Subject),
                BodyText = Sanitize(context.App, "email.bodyText", email.BodyText),
                Attachments = email.Attachments,
                MentionsAttachment = email.MentionsAttachment,
            };
        }

        var security = context.Security is null
            ? new SecurityContext { RedactionApplied = true }
            : new SecurityContext
            {
                TenantId = context.Security.TenantId,
                ProcessingLocation = context.Security.ProcessingLocation,
                RedactionApplied = true,
            };

        return new AuraContext
        {
            App = context.App,
            Window = context.Window,
            Scope = context.Scope,
            Trigger = context.Trigger,
            Email = email,
            Document = context.Document,
            User = context.User,
            Session = context.Session,
            Security = security,
            Environment = context.Environment,
            CapturedAt = context.CapturedAt,
            SensitiveRedacted = true,
        };
    }

    private string Sanitize(string app, string field, string text)
    {
        if (IsZoned(app, field))
        {
            return ZoneRedactionText;
        }

        var matches = _detector.Detect(text);
        return matches.Count == 0 ? text : _redactor.Redact(text, matches);
    }

    private bool IsZoned(string app, string field) => _zones.Any(zone =>
        (zone.App == "*" || string.Equals(zone.App, app, StringComparison.OrdinalIgnoreCase)) &&
        string.Equals(zone.Field, field, StringComparison.OrdinalIgnoreCase));
}
