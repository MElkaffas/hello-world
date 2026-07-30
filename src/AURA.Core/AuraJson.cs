using System.Text.Json;

namespace Aura.Core;

/// <summary>Shared JSON options - camelCase so field paths in rules ("email.bodyText") match serialized Context.</summary>
public static class AuraJson
{
    public static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        ReadCommentHandling = JsonCommentHandling.Skip,
        AllowTrailingCommas = true,
    };
}
