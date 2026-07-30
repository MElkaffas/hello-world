using Aura.Core.Models;
using Aura.Core.Policy;

namespace Aura.Core.Actions;

public enum SuggestionState
{
    Proposed,
    Approved,
    Applied,
    Verified,
    Ignored,
    Failed,
}

/// <summary>
/// The user-facing unit of the Suggest → Preview → Approve → Verify flow (README §7).
/// Findings never mutate anything directly - only a Suggestion, once approved, does.
/// </summary>
public sealed class Suggestion
{
    public Guid Id { get; init; } = Guid.NewGuid();
    public required Finding Finding { get; init; }
    public required PolicyDecision Decision { get; init; }
    public SuggestionState State { get; set; } = SuggestionState.Proposed;
    public string? PreviewDescription { get; set; }
    public string? VerificationMessage { get; set; }
    public DateTimeOffset CreatedAt { get; init; } = DateTimeOffset.UtcNow;
}
