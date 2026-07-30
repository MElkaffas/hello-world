using Aura.Core.Models;

namespace Aura.Core.Rules;

public interface IRuleEngine
{
    IReadOnlyList<Rule> Rules { get; }

    void LoadRules(IEnumerable<Rule> rules);
    void LoadRulesFromJson(string json);
    void LoadRulesFromDirectory(string directoryPath);

    /// <summary>Evaluates every loaded rule whose trigger matches the context. Never references a specific app.</summary>
    IReadOnlyList<Finding> Evaluate(AuraContext context);
}
