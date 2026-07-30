using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using Aura.Core.Models;

namespace Aura.Core.Rules;

public static class ConditionEvaluator
{
    public static bool Evaluate(JsonNode? root, RuleCondition condition)
    {
        var value = FieldPathResolver.Resolve(root, condition.Field);

        return condition.Op switch
        {
            "isEmpty" => IsEmpty(value),
            "isNotEmpty" => !IsEmpty(value),
            "equals" => JsonNode.DeepEquals(value, condition.Value),
            "notEquals" => !JsonNode.DeepEquals(value, condition.Value),
            "contains" => Contains(value, condition.Value),
            "notContains" => !Contains(value, condition.Value),
            "greaterThan" => Compare(value, condition.Value) > 0,
            "lessThan" => Compare(value, condition.Value) < 0,
            "matchesRegex" => MatchesRegex(value, condition.Value),
            "in" => In(value, condition.Value),
            _ => throw new NotSupportedException($"Unknown condition operator '{condition.Op}'."),
        };
    }

    private static bool IsEmpty(JsonNode? value)
    {
        if (value is null) return true;

        return value.GetValueKind() switch
        {
            JsonValueKind.Null or JsonValueKind.Undefined => true,
            JsonValueKind.String => string.IsNullOrEmpty(value.GetValue<string>()),
            JsonValueKind.Array => ((JsonArray)value).Count == 0,
            JsonValueKind.Object => ((JsonObject)value).Count == 0,
            _ => false,
        };
    }

    private static bool Contains(JsonNode? value, JsonNode? target)
    {
        if (value is JsonArray array)
        {
            return array.Any(item => JsonNode.DeepEquals(item, target));
        }

        if (value is not null && target is not null && value.GetValueKind() == JsonValueKind.String)
        {
            var haystack = value.GetValue<string>();
            var needle = target.GetValueKind() == JsonValueKind.String ? target.GetValue<string>() : target.ToJsonString();
            return haystack.Contains(needle, StringComparison.OrdinalIgnoreCase);
        }

        return false;
    }

    private static int Compare(JsonNode? value, JsonNode? target)
    {
        var left = value is not null && value.GetValueKind() == JsonValueKind.Number ? value.GetValue<double>() : double.NaN;
        var right = target is not null && target.GetValueKind() == JsonValueKind.Number ? target.GetValue<double>() : double.NaN;
        return left.CompareTo(right);
    }

    private static bool MatchesRegex(JsonNode? value, JsonNode? pattern)
    {
        if (value is null || pattern is null) return false;
        if (value.GetValueKind() != JsonValueKind.String || pattern.GetValueKind() != JsonValueKind.String) return false;
        return Regex.IsMatch(value.GetValue<string>(), pattern.GetValue<string>());
    }

    private static bool In(JsonNode? value, JsonNode? candidates)
    {
        return candidates is JsonArray array && array.Any(item => JsonNode.DeepEquals(item, value));
    }
}
