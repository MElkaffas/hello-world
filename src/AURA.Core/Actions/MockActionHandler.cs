namespace Aura.Core.Actions;

/// <summary>
/// Applies suggested actions against an in-memory stand-in instead of a live app - lets the
/// full Suggest → Preview → Approve → Verify → Audit loop be exercised before a real connector
/// (Outlook, Word) is wired in.
/// </summary>
public sealed class MockActionHandler : IActionHandler
{
    public MockActionHandler(string app)
    {
        App = app;
    }

    public string App { get; }

    public Task<string> PreviewAsync(Suggestion suggestion, CancellationToken cancellationToken = default)
    {
        var action = suggestion.Finding.Rule.SuggestedAction;
        var description = action is null
            ? "No automated fix available - manual review required."
            : action.Type switch
            {
                "setField" => $"Set '{action.Field}' to the suggested value.",
                _ => $"Apply action '{action.Type}'.",
            };

        return Task.FromResult(description);
    }

    public Task ApplyAsync(Suggestion suggestion, CancellationToken cancellationToken = default)
    {
        var action = suggestion.Finding.Rule.SuggestedAction;
        suggestion.VerificationMessage = action?.Field is string field
            ? $"'{field}' updated."
            : $"'{suggestion.Finding.Rule.Name}' resolved.";

        return Task.CompletedTask;
    }

    public Task<bool> VerifyAsync(Suggestion suggestion, CancellationToken cancellationToken = default) =>
        Task.FromResult(true);
}
