using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text.Json.Serialization;
using Aura.Core.Actions;
using Aura.Core.Connectors;
using Aura.Core.Models;
using Azure.Core;
using Azure.Identity;

namespace Aura.Connectors.Outlook;

/// <summary>
/// Reads/writes real Outlook mail objects via Microsoft Graph (README §4, §12), operating on
/// the user's latest draft message. Graph only sees saved mail - it can't see keystrokes in a
/// still-open compose window; that half of the live experience needs an Outlook Add-in
/// (Office.js) feeding the same Context/Rule/Policy pipeline, which is the natural next step
/// once this connector is wired into the app.
/// </summary>
public sealed class OutlookGraphConnector : IConnector, IActionHandler
{
    private static readonly string[] Scopes = { "https://graph.microsoft.com/Mail.ReadWrite" };
    private const string GraphBaseUrl = "https://graph.microsoft.com/v1.0";

    private readonly HttpClient _httpClient;
    private readonly TokenCredential _credential;
    private string? _lastMessageId;

    public OutlookGraphConnector(HttpClient httpClient, TokenCredential? credential = null)
    {
        _httpClient = httpClient;
        _credential = credential ?? new InteractiveBrowserCredential();
    }

    public string App => "outlook";

    public async Task<AuraContext> CaptureAsync(CancellationToken cancellationToken = default)
    {
        await AuthorizeAsync(cancellationToken);

        var response = await _httpClient.GetAsync(
            $"{GraphBaseUrl}/me/mailFolders/drafts/messages?$top=1&$orderby=lastModifiedDateTime desc" +
            "&$select=id,subject,body,toRecipients,ccRecipients,hasAttachments",
            cancellationToken);
        response.EnsureSuccessStatusCode();

        var page = await response.Content.ReadFromJsonAsync<GraphMessageListResponse>(cancellationToken: cancellationToken)
            ?? new GraphMessageListResponse();
        var message = page.Value.FirstOrDefault();

        if (message is null)
        {
            return new AuraContext { App = App, Window = "Drafts", Trigger = "email.compose", Email = new EmailContext() };
        }

        _lastMessageId = message.Id;
        var bodyText = StripHtml(message.Body?.Content ?? string.Empty);

        return new AuraContext
        {
            App = App,
            Window = "Compose",
            Scope = "document",
            Trigger = "email.compose",
            Email = new EmailContext
            {
                To = message.ToRecipients.Select(r => r.EmailAddress?.Address ?? string.Empty).ToArray(),
                Cc = message.CcRecipients.Select(r => r.EmailAddress?.Address ?? string.Empty).ToArray(),
                Subject = message.Subject ?? string.Empty,
                BodyText = bodyText,
                MentionsAttachment = bodyText.Contains("attach", StringComparison.OrdinalIgnoreCase),
                Attachments = message.HasAttachments ? new[] { "(unspecified)" } : Array.Empty<string>(),
            },
        };
    }

    public Task<string> PreviewAsync(Suggestion suggestion, CancellationToken cancellationToken = default)
    {
        var action = suggestion.Finding.Rule.SuggestedAction;
        var description = action?.Field switch
        {
            "email.subject" => "Set the draft's Subject.",
            "email.attachments" => "Remind you to attach the missing file before sending.",
            _ => $"Apply '{action?.Type}'.",
        };

        return Task.FromResult(description);
    }

    public async Task ApplyAsync(Suggestion suggestion, CancellationToken cancellationToken = default)
    {
        var action = suggestion.Finding.Rule.SuggestedAction;
        if (action?.Field != "email.subject" || _lastMessageId is null)
        {
            suggestion.VerificationMessage = "No automatic fix available for this field yet.";
            return;
        }

        await AuthorizeAsync(cancellationToken);

        var suggestedSubject = action.Value?.GetValue<string>() ?? "(no subject)";
        var payload = new { subject = suggestedSubject };

        var request = new HttpRequestMessage(HttpMethod.Patch, $"{GraphBaseUrl}/me/messages/{_lastMessageId}")
        {
            Content = JsonContent.Create(payload),
        };
        var response = await _httpClient.SendAsync(request, cancellationToken);
        response.EnsureSuccessStatusCode();

        suggestion.VerificationMessage = $"Subject set to '{suggestedSubject}'.";
    }

    public async Task<bool> VerifyAsync(Suggestion suggestion, CancellationToken cancellationToken = default)
    {
        if (_lastMessageId is null)
        {
            return false;
        }

        await AuthorizeAsync(cancellationToken);
        var response = await _httpClient.GetAsync(
            $"{GraphBaseUrl}/me/messages/{_lastMessageId}?$select=subject", cancellationToken);
        if (!response.IsSuccessStatusCode)
        {
            return false;
        }

        var message = await response.Content.ReadFromJsonAsync<GraphMessage>(cancellationToken: cancellationToken);
        return !string.IsNullOrEmpty(message?.Subject);
    }

    private async Task AuthorizeAsync(CancellationToken cancellationToken)
    {
        var token = await _credential.GetTokenAsync(new TokenRequestContext(Scopes), cancellationToken);
        _httpClient.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", token.Token);
    }

    private static string StripHtml(string html) =>
        System.Text.RegularExpressions.Regex.Replace(html, "<[^>]+>", " ").Trim();

    private sealed class GraphMessageListResponse
    {
        [JsonPropertyName("value")]
        public List<GraphMessage> Value { get; init; } = new();
    }

    private sealed class GraphMessage
    {
        [JsonPropertyName("id")]
        public string? Id { get; init; }

        [JsonPropertyName("subject")]
        public string? Subject { get; init; }

        [JsonPropertyName("body")]
        public GraphBody? Body { get; init; }

        [JsonPropertyName("toRecipients")]
        public List<GraphRecipient> ToRecipients { get; init; } = new();

        [JsonPropertyName("ccRecipients")]
        public List<GraphRecipient> CcRecipients { get; init; } = new();

        [JsonPropertyName("hasAttachments")]
        public bool HasAttachments { get; init; }
    }

    private sealed class GraphBody
    {
        [JsonPropertyName("contentType")]
        public string? ContentType { get; init; }

        [JsonPropertyName("content")]
        public string? Content { get; init; }
    }

    private sealed class GraphRecipient
    {
        [JsonPropertyName("emailAddress")]
        public GraphEmailAddress? EmailAddress { get; init; }
    }

    private sealed class GraphEmailAddress
    {
        [JsonPropertyName("address")]
        public string? Address { get; init; }

        [JsonPropertyName("name")]
        public string? Name { get; init; }
    }
}
