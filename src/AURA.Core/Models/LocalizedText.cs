namespace Aura.Core.Models;

/// <summary>Bilingual AR/EN copy, per README §1/§14 RTL-aware requirement.</summary>
public sealed class LocalizedText
{
    public string? Ar { get; init; }
    public string? En { get; init; }

    public string Get(string lang) =>
        string.Equals(lang, "ar", StringComparison.OrdinalIgnoreCase)
            ? Ar ?? En ?? string.Empty
            : En ?? Ar ?? string.Empty;
}
