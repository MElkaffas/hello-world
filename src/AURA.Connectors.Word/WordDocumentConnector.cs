using Aura.Core.Connectors;
using Aura.Core.Models;
using Aura.Core.Profiles;
using DocumentFormat.OpenXml.Packaging;
using DocumentFormat.OpenXml.Wordprocessing;

namespace Aura.Connectors.Word;

/// <summary>
/// Reads a Word document's actual paragraph styles via the OpenXML SDK (README §4) and scores
/// them against the active Document Standard Profile (README §13) - replacing the prototype's
/// hard-coded font check. Read-only for now; writing corrected styles back into the .docx is
/// the natural next step once this is wired into the app (mirrors the Outlook connector, which
/// already implements the write/verify side against Graph).
/// </summary>
public sealed class WordDocumentConnector : IConnector
{
    private readonly string _filePath;
    private readonly DocumentStandardProfile _profile;

    public WordDocumentConnector(string filePath, DocumentStandardProfile profile)
    {
        _filePath = filePath;
        _profile = profile;
    }

    public string App => "word";

    public Task<AuraContext> CaptureAsync(CancellationToken cancellationToken = default)
    {
        using var document = WordprocessingDocument.Open(_filePath, isEditable: false);
        var body = document.MainDocumentPart?.Document?.Body
            ?? throw new InvalidOperationException($"'{_filePath}' has no document body.");

        var paragraphs = body.Elements<Paragraph>().ToList();
        var styles = new List<DocumentStyleRun>();

        for (var i = 0; i < paragraphs.Count; i++)
        {
            styles.Add(ReadStyleRun(paragraphs[i], role: i switch
            {
                0 => "title",
                1 => "subtitle",
                _ => "body",
            }));
        }

        var rawDocument = new DocumentContext { ProfileName = _profile.Name, Styles = styles };
        var compliance = DocumentProfileChecker.Check(rawDocument, _profile);

        var enrichedDocument = new DocumentContext
        {
            ProfileName = _profile.Name,
            Styles = styles,
            ComplianceScore = compliance.Score,
            ComplianceIssues = compliance.Issues,
        };

        return Task.FromResult(new AuraContext
        {
            App = App,
            Window = Path.GetFileName(_filePath),
            Scope = "document",
            Trigger = "document.edit",
            Document = enrichedDocument,
        });
    }

    private static DocumentStyleRun ReadStyleRun(Paragraph paragraph, string role)
    {
        var runProperties = paragraph.Elements<Run>().FirstOrDefault()?.RunProperties;

        var font = runProperties?.RunFonts?.Ascii?.Value ?? "Calibri";

        var halfPoints = runProperties?.FontSize?.Val?.Value;
        var size = halfPoints is not null && double.TryParse(halfPoints, out var parsedSize) ? parsedSize / 2.0 : 11.0;

        var bold = runProperties?.Bold is not null
            && (runProperties.Bold.Val is null || runProperties.Bold.Val.Value);

        var alignment = paragraph.ParagraphProperties?.Justification?.Val?.Value switch
        {
            JustificationValues.Both => "Justified",
            JustificationValues.Center => "Center",
            JustificationValues.Right => "Right",
            JustificationValues.Left => "Left",
            _ => null,
        };

        return new DocumentStyleRun { Role = role, Font = font, Size = size, Bold = bold, Alignment = alignment };
    }
}
