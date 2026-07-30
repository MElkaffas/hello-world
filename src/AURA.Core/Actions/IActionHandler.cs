namespace Aura.Core.Actions;

/// <summary>
/// Connector-specific counterpart to <see cref="Connectors.IConnector"/>: knows how to actually
/// preview, apply, and verify a rule's suggested action against the live app. The pipeline
/// only ever calls through this interface, never a concrete Outlook/Word type (README §7).
/// </summary>
public interface IActionHandler
{
    string App { get; }

    /// <summary>Human-readable description of what will change, shown before the user approves.</summary>
    Task<string> PreviewAsync(Suggestion suggestion, CancellationToken cancellationToken = default);

    /// <summary>Actually mutates the live app/document. Only called after explicit approval
    /// (or when policy grants Auto-Act).</summary>
    Task ApplyAsync(Suggestion suggestion, CancellationToken cancellationToken = default);

    /// <summary>Re-reads the app/document to confirm the fix took effect.</summary>
    Task<bool> VerifyAsync(Suggestion suggestion, CancellationToken cancellationToken = default);
}
