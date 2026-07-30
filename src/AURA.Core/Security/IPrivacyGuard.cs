using Aura.Core.Models;

namespace Aura.Core.Security;

/// <summary>
/// The pipeline step run before Rule Engine evaluation, per README §10's
/// Capture → Privacy Zones → PII Detection → Redaction → Context order: excludes zoned fields
/// entirely, then redacts detected sensitive data from what's left.
/// </summary>
public interface IPrivacyGuard
{
    AuraContext Apply(AuraContext context);
}
