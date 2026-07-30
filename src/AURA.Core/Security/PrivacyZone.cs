namespace Aura.Core.Security;

/// <summary>
/// A field that must never leave this process in readable form, even inside an otherwise
/// monitored app (README §10) - e.g. password fields, bank/card numbers, private mail. The
/// whole field is replaced wholesale, not scanned for partial matches like PII detection does.
/// </summary>
public sealed class PrivacyZone
{
    public required string Name { get; init; }

    /// <summary>App this zone applies to, or "*" for all apps.</summary>
    public string App { get; init; } = "*";

    /// <summary>Dotted field path into the serialized Context (same syntax as
    /// <c>RuleCondition.Field</c>), e.g. "email.bodyText".</summary>
    public required string Field { get; init; }
}
