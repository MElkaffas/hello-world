using System.IO;
using System.Windows;
using Aura.Core.Actions;
using Aura.Core.Audit;
using Aura.Core.Confidence;
using Aura.Core.Connectors;
using Aura.Core.Pipeline;
using Aura.Core.Policy;
using Aura.Core.Rules;
using Aura.Desktop.Views;

namespace Aura.Desktop.Services;

/// <summary>
/// Composition root wiring the Core pipeline (README §6-9) to the tray, avatar, control panel,
/// and alert bubble. Uses a mock connector for now (README build order: "wire the pipeline
/// against a mock connector before integrating Graph API").
/// </summary>
public sealed class AssistantController : IDisposable
{
    private readonly DetectionPipeline _pipeline;
    private readonly FileAuditLog _auditLog;
    private readonly TrayIconService _tray;
    private readonly AvatarWindow _avatar;
    private readonly ControlPanelWindow _controlPanel;

    private AlertBubbleWindow? _alertWindow;
    private readonly List<Suggestion> _pending = new();
    private int _currentIndex;

    private AssistantMode _mode = AssistantMode.Idle;
    private string _language = "en";
    private MonitoringScope _scope = MonitoringScope.Region;

    public AssistantController()
    {
        var ruleEngine = new RuleEngine();
        ruleEngine.LoadRulesFromDirectory(Path.Combine(AppContext.BaseDirectory, "Rules"));

        var confidenceEngine = new ConfidenceEngine();

        var policyEngine = new PolicyEngine();
        policyEngine.Configure(new[] { new PolicyRule { App = "*", Ceiling = AutonomyLevel.AskAndAct } });

        var auditPath = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "AURA", "audit.jsonl");
        _auditLog = new FileAuditLog(auditPath);

        _pipeline = new DetectionPipeline(
            ruleEngine,
            confidenceEngine,
            policyEngine,
            _auditLog,
            new IActionHandler[] { new MockActionHandler("outlook"), new MockActionHandler("word") });

        _avatar = new AvatarWindow();
        _controlPanel = new ControlPanelWindow();
        _tray = new TrayIconService();

        WireEvents();
    }

    public void Start()
    {
        _avatar.Left = SystemParameters.PrimaryScreenWidth - 200;
        _avatar.Top = SystemParameters.PrimaryScreenHeight - 260;
        _avatar.Show();

        _controlPanel.Left = 24;
        _controlPanel.Top = 24;
        _controlPanel.SetStatus("Idle. Choose a scenario and press Start.");
        _controlPanel.SetPrivacyStatus("Nothing monitored yet.");
        _controlPanel.Show();
    }

    private void WireEvents()
    {
        _tray.StartStopClicked += (_, _) => ToggleWatching();
        _tray.SelectAreaClicked += (_, _) => SelectArea();
        _tray.OpenControlPanelClicked += (_, _) => { _controlPanel.Show(); _controlPanel.Activate(); };
        _tray.AssistantStyleSelected += (_, style) => _avatar.SetStyle(style);
        _tray.LanguageSelected += (_, lang) => ApplyLanguage(lang);
        _tray.ExitClicked += (_, _) => Application.Current.Shutdown();

        _controlPanel.LanguageChanged += (_, lang) => ApplyLanguage(lang);
        _controlPanel.StyleChanged += (_, style) => _avatar.SetStyle(style);
        _controlPanel.ScopeChanged += (_, scope) => _scope = scope;
        _controlPanel.SelectAreaRequested += (_, _) => SelectArea();
        _controlPanel.StartStopRequested += (_, _) => ToggleWatching();
        _controlPanel.DeleteDataRequested += async (_, _) => await DeleteCapturedDataAsync();
        _controlPanel.RefreshAuditRequested += async (_, _) => await RefreshAuditAsync();
    }

    private void ApplyLanguage(string lang)
    {
        _language = lang;
        _controlPanel.FlowDirection = lang == "ar" ? FlowDirection.RightToLeft : FlowDirection.LeftToRight;
    }

    private void SelectArea()
    {
        var picker = new RegionSelectionWindow();
        var accepted = picker.ShowDialog();

        if (accepted == true && picker.SelectedRegion is Rect region)
        {
            _scope = MonitoringScope.Region;
            _controlPanel.SetStatus($"Region set: {(int)region.Width}x{(int)region.Height}.");
        }
    }

    private void ToggleWatching()
    {
        if (_mode == AssistantMode.Watching)
        {
            StopWatching();
        }
        else
        {
            _ = BeginWatchingAsync();
        }
    }

    private async Task BeginWatchingAsync()
    {
        _mode = AssistantMode.Watching;
        _avatar.SetMode(AssistantMode.Watching);
        _avatar.SetMood(RobotMood.Thinking);
        _controlPanel.SetStartStopLabel("Stop");
        _controlPanel.SetStatus("Thinking…");

        var app = _controlPanel.IsOutlookScenarioSelected ? "outlook" : "word";
        _controlPanel.SetPrivacyStatus($"{app} · {_scope} · Local AI");

        await Task.Delay(TimeSpan.FromMilliseconds(1600));

        var connector = _controlPanel.IsOutlookScenarioSelected
            ? MockConnector.OutlookComposeMissingSubjectAndAttachment()
            : MockConnector.WordDocumentOffProfile();

        var context = await connector.CaptureAsync();
        var suggestions = _pipeline.Process(context);

        _pending.Clear();
        _pending.AddRange(suggestions);
        _currentIndex = 0;

        if (_pending.Count == 0)
        {
            _avatar.SetMood(RobotMood.Happy);
            _controlPanel.SetStatus("No issues found.");
            await Task.Delay(TimeSpan.FromMilliseconds(1200));
            _avatar.SetMood(RobotMood.Idle);
        }
        else
        {
            _avatar.SetMood(RobotMood.Alert);
            _controlPanel.SetStatus($"{_pending.Count} suggestion(s) found.");
            await ShowCurrentSuggestionAsync();
        }
    }

    private void StopWatching()
    {
        _mode = AssistantMode.Idle;
        _avatar.SetMode(AssistantMode.Idle);
        _avatar.SetMood(RobotMood.Idle);
        _controlPanel.SetStartStopLabel("Start");
        _controlPanel.SetStatus("Stopped.");
        _controlPanel.SetPrivacyStatus("Nothing monitored.");
        CloseAlert();
        _pending.Clear();
    }

    private async Task ShowCurrentSuggestionAsync()
    {
        if (_currentIndex < 0 || _currentIndex >= _pending.Count)
        {
            CloseAlert();
            return;
        }

        var suggestion = _pending[_currentIndex];
        var preview = await _pipeline.PreviewAsync(suggestion);
        var message = suggestion.Finding.Rule.Message?.Get(_language) ?? suggestion.Finding.Rule.Name;

        _alertWindow ??= CreateAlertWindow();
        _alertWindow.Display(message, preview, _currentIndex + 1, _pending.Count);
        PositionAlertNearAvatar();
        _alertWindow.Show();
        _alertWindow.Activate();
    }

    private AlertBubbleWindow CreateAlertWindow()
    {
        var window = new AlertBubbleWindow();
        window.ApplyRequested += async (_, _) => await ApplyCurrentAsync();
        window.IgnoreRequested += async (_, _) => await IgnoreCurrentAsync();
        window.NextRequested += (_, _) => _ = AdvanceAsync();
        return window;
    }

    private async Task ApplyCurrentAsync()
    {
        if (_alertWindow is null || _currentIndex >= _pending.Count)
        {
            return;
        }

        _alertWindow.SetBusy(true);
        var suggestion = _pending[_currentIndex];
        await _pipeline.ApproveAsync(suggestion, approvedBy: Environment.UserName);
        _avatar.SetMood(RobotMood.Happy);
        _alertWindow.SetBusy(false);
        await RemoveCurrentAndAdvanceAsync();
    }

    private async Task IgnoreCurrentAsync()
    {
        if (_alertWindow is null || _currentIndex >= _pending.Count)
        {
            return;
        }

        var suggestion = _pending[_currentIndex];
        await _pipeline.IgnoreAsync(suggestion);
        await RemoveCurrentAndAdvanceAsync();
    }

    private async Task AdvanceAsync()
    {
        _currentIndex++;
        if (_currentIndex >= _pending.Count)
        {
            CloseAlert();
            _avatar.SetMood(RobotMood.Idle);
            _controlPanel.SetStatus("All suggestions reviewed.");
        }
        else
        {
            await ShowCurrentSuggestionAsync();
        }
    }

    private async Task RemoveCurrentAndAdvanceAsync()
    {
        _pending.RemoveAt(_currentIndex);

        if (_pending.Count == 0)
        {
            CloseAlert();
            _controlPanel.SetStatus("All suggestions reviewed.");
            await Task.Delay(TimeSpan.FromMilliseconds(900));
            _avatar.SetMood(RobotMood.Idle);
        }
        else
        {
            if (_currentIndex >= _pending.Count)
            {
                _currentIndex = _pending.Count - 1;
            }

            await ShowCurrentSuggestionAsync();
        }
    }

    private void PositionAlertNearAvatar()
    {
        if (_alertWindow is null)
        {
            return;
        }

        _alertWindow.Left = _avatar.Left - _alertWindow.Width - 16;
        _alertWindow.Top = _avatar.Top;
    }

    private void CloseAlert() => _alertWindow?.Hide();

    private async Task DeleteCapturedDataAsync()
    {
        var path = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "AURA", "audit.jsonl");

        if (File.Exists(path))
        {
            File.Delete(path);
        }

        _controlPanel.SetAuditEntries(Array.Empty<string>());
        _controlPanel.SetPrivacyStatus("Captured data deleted.");
        await Task.CompletedTask;
    }

    private async Task RefreshAuditAsync()
    {
        var entries = await _auditLog.ReadAllAsync();
        _controlPanel.SetAuditEntries(entries.Select(entry =>
            $"{entry.Timestamp:g} · {entry.App} · {entry.RuleName} · {entry.Action} · by {entry.ApprovedBy}"));
    }

    public void Dispose() => _tray.Dispose();
}
