using System.Text.Json.Nodes;

namespace Aura.Core.Rules;

/// <summary>
/// Resolves dotted field paths (e.g. "email.bodyText", "document.styles[0].font")
/// against the JSON-serialized Context object, so Rule conditions never need to know
/// about the underlying C# type shape.
/// </summary>
public static class FieldPathResolver
{
    public static JsonNode? Resolve(JsonNode? root, string path)
    {
        JsonNode? current = root;

        foreach (var (name, index) in Tokenize(path))
        {
            if (current is JsonObject obj)
            {
                obj.TryGetPropertyValue(name, out current);
            }
            else
            {
                return null;
            }

            if (index is int idx)
            {
                current = current is JsonArray arr && idx >= 0 && idx < arr.Count ? arr[idx] : null;
            }
        }

        return current;
    }

    private static IEnumerable<(string Name, int? Index)> Tokenize(string path)
    {
        foreach (var part in path.Split('.', StringSplitOptions.RemoveEmptyEntries))
        {
            var bracket = part.IndexOf('[');
            if (bracket >= 0 && part.EndsWith(']'))
            {
                var name = part[..bracket];
                var indexText = part[(bracket + 1)..^1];
                yield return (name, int.Parse(indexText));
            }
            else
            {
                yield return (part, null);
            }
        }
    }
}
