using System.Text.Json;
using System.Text.Json.Nodes;
using Aura.Core.Models;

namespace Aura.Core.Rules;

/// <summary>
/// Evaluates data-driven <see cref="Rule"/> definitions against a normalized <see cref="AuraContext"/>.
/// This class is deliberately app-agnostic (README §6, §11): it has no knowledge of Outlook, Word,
/// or any other connector. Everything it needs comes from the Context object and the loaded rule JSON.
/// </summary>
public sealed class RuleEngine : IRuleEngine
{
    private readonly List<Rule> _rules = new();

    public IReadOnlyList<Rule> Rules => _rules;

    public void LoadRules(IEnumerable<Rule> rules)
    {
        _rules.AddRange(rules);
    }

    public void LoadRulesFromJson(string json)
    {
        var rules = JsonSerializer.Deserialize<List<Rule>>(json, AuraJson.Options)
            ?? throw new InvalidOperationException("Rule JSON deserialized to null.");
        LoadRules(rules);
    }

    public void LoadRulesFromDirectory(string directoryPath)
    {
        foreach (var file in Directory.EnumerateFiles(directoryPath, "*.json", SearchOption.TopDirectoryOnly).OrderBy(f => f))
        {
            var json = File.ReadAllText(file);
            var trimmed = json.TrimStart();
            if (trimmed.StartsWith('['))
            {
                LoadRulesFromJson(json);
            }
            else
            {
                var rule = JsonSerializer.Deserialize<Rule>(json, AuraJson.Options)
                    ?? throw new InvalidOperationException($"Rule file '{file}' deserialized to null.");
                LoadRules(new[] { rule });
            }
        }
    }

    public IReadOnlyList<Finding> Evaluate(AuraContext context)
    {
        var root = JsonSerializer.SerializeToNode(context, AuraJson.Options);
        var findings = new List<Finding>();

        foreach (var rule in _rules)
        {
            if (!MatchesTrigger(rule.Trigger, context.Trigger)) continue;
            if (rule.Conditions.Count == 0) continue;

            var allMatch = rule.Conditions.All(condition => ConditionEvaluator.Evaluate(root, condition));
            if (allMatch)
            {
                findings.Add(new Finding { Rule = rule, Context = context });
            }
        }

        return findings;
    }

    private static bool MatchesTrigger(string ruleTrigger, string contextTrigger) =>
        ruleTrigger == "*" || string.Equals(ruleTrigger, contextTrigger, StringComparison.OrdinalIgnoreCase);
}
