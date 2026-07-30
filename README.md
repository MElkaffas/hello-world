# AURA — AI Unified Remote Assistant

A native Windows desktop AI agent that watches a scoped region/window/app, understands what's
there via a normalized Context object, and offers **Suggest → Preview → Approve → Verify**
fixes for things like an empty email Subject or a Word document that doesn't match its house
style. The full product spec — architecture, detection pipeline, data models, privacy model,
and design tokens — lives in [`design_handoff_aura/README.md`](design_handoff_aura/README.md);
this README covers the engineering side: what's built, how to build/run it, and how to extend
it.

The one rule the whole codebase is built around: **`AURA.Core` never says `if (app ==
"outlook")`.** Connectors normalize whatever app they're watching into a `Context` object; a
data-driven **Rule Engine** evaluates JSON rules against that Context; a **Confidence Engine**
scores findings; a **Policy Engine** decides how loud to be. Core has no idea Outlook or Word
exist.

## Solution layout

```
AURA.sln
src/
  AURA.Core/                  Context, Rule Engine, Confidence Engine, Policy Engine,
                              Connector/ActionHandler interfaces, mock connector, IAuditSink,
                              Document Standard Profile checker. Plain net8.0 - no Windows
                              dependency, builds and tests on any OS. Has NO knowledge of any
                              specific app, tenant, or storage choice - see "Architecture" below.
  AURA.Host/                   Composition root: turns an AuraConfiguration (tenant.json) +
                              ConnectorRegistry into a ready-to-use DetectionPipeline. Plain
                              net8.0. This is what used to be hardcoded inside
                              AssistantController's constructor.
  AURA.Desktop/                WPF tray app + floating robot avatar + control panel + alert
                              bubble (net8.0-windows). Owns UI state only - it asks App.xaml.cs
                              for an already-built AuraHost, it doesn't build one itself.
  AURA.Connectors.Outlook/     Reads/writes Outlook drafts via Microsoft Graph REST + Azure.Identity.
  AURA.Connectors.Word/        Reads a .docx's real styles via the OpenXML SDK and scores them
                              against a Document Standard Profile.
                              (Not yet registered in AURA.Host - see "What's implemented".)
tests/
  AURA.Core.Tests/             xunit unit + integration tests for the Core pipeline.
  AURA.Host.Tests/             xunit tests for configuration loading, the connector registry,
                              and AuraHost.Build's end-to-end wiring.
samples/
  rules/                       Example JSON rules (README §11 schema).
  profiles/                    Example Document Standard Profile JSON (README §13 schema).
  host/                        Example tenant.json (AURA.Host.Configuration schema).
design_handoff_aura/           The original design handoff package (spec + HTML prototype),
                              copied in verbatim as the source of truth for visual design.
```

## Architecture

This went through a full architecture review before this round of changes (see conversation
history / commit log) - the short version of what changed and why:

- **`AssistantController` used to be a god object**: it built the Rule/Confidence/Policy
  engines, the audit log, and two mock connectors in its constructor, *and* owned every bit of
  UI state. It now only owns UI state and windows. All service construction moved to
  **`AuraHost.Build(configuration, registry)`** in the new `AURA.Host` project - `App.xaml.cs`
  builds one `AuraHost` at startup and hands it to `AssistantController` via the constructor.
- **`IAuditLog`/`FileAuditLog` were renamed to `IAuditSink`/`LocalFileAuditSink`** (not
  duplicated) - same contract, plus a `DeleteAllAsync` method so "delete captured data" goes
  through the abstraction instead of a UI-layer file path. The old doc comment overclaimed
  "tamper-evident"; the new one says plainly that this local-file implementation is *not*
  tamper-evident or encrypted, and that a real deployment needs a different `IAuditSink`.
- **Connectors are now resolved through a `ConnectorRegistry`** instead of being `new`'d up by
  name in a constructor. `IConnectorFactory` is the plugin seam; today only the two mock
  factories are registered (`Aura.Host.Connectors.MockConnectorFactories.cs`) - proving the
  mechanism, not yet a DLL-loading plugin system.
- **Configuration is now a file** (`tenant.json`, loaded via `AuraConfigurationLoader`) instead
  of hardcoded values in `AssistantController`: which rules directory, which policy ceilings,
  which connectors are enabled. Still single-tenant/file-based on purpose - multi-tenancy
  belongs in a future server/backend, not inside the Desktop Agent itself.
- **`AuraContext` gained `User`/`Session`/`Security`/`Environment`** as optional nested objects,
  additively - `Email`/`Document` and every existing rule/test that references them are
  untouched.

**Phase 2 additions (Security/Privacy, done before any AI work per the agreed order):**

- **`Aura.Core.Security`** (a namespace inside `AURA.Core`, not yet its own project - see note
  below): `PrivacyZone`, `ISensitiveDataDetector` + a regex-based default (email/national-ID/
  credit-card/phone patterns), `IRedactor` + a default that merges overlapping matches before
  replacing them, and `IPrivacyGuard` combining all three into one context-sanitizing step.
- **`DetectionPipeline.Process`** now runs `IPrivacyGuard.Apply` *before* the Rule Engine sees
  the context (README §10's Capture → Privacy Zones → PII Detection → Redaction → Context
  order). The guard is an optional constructor parameter (defaults to none/pass-through) so
  every existing test and call site kept compiling unchanged; `AuraHost.Build` always supplies
  a real one.
- **`tenant.json` gained `privacyZones` and `sessionTimeLimitMinutes`.** Session-scoped
  monitoring (README §10 - "Monitor Outlook for 30 min" auto-stop) is enforced in
  `AssistantController` via a `DispatcherTimer` keyed off the configured limit.
- The mock Outlook scenario's body text now contains a real-looking email address and phone
  number specifically so redaction has something genuine to catch when you run the app.

*Why Security lives inside `AURA.Core` rather than a separate `AURA.Core.Security` project (as
sketched in the original review): a regex-based detector/redactor is a handful of small classes,
not a subsystem - splitting it into its own project now would be premature. If/when a heavier
detector (ML-based, a Presidio-style service) replaces the regex one, that's the point to give
it its own project, mirroring how Connectors already work.*

Deliberately **not** done in this pass (next up per the agreed phase order): Data Classification
(a broader taxonomy beyond these four PII categories), Consent management, Secure Storage
(encrypting the audit log at rest), Secret Management (credential vaulting - not needed while
only mocks are wired), a full Permission Model/RBAC, an AI Gateway or any LLM call, screen
understanding (UIA/OCR/Vision), a no-code Rule Builder, or a plugin/DLL-loading connector model.

## Build & run

- **`AURA.Core`, `AURA.Host`, and their test projects** target plain `net8.0` and have no
  Windows dependency — they build and `dotnet test` on Linux/macOS/Windows/CI.
- **`AURA.Desktop`** targets `net8.0-windows` (WPF + WinForms for the tray icon) and can only be
  built/run on Windows with the `Microsoft.NET.Sdk.WindowsDesktop` workload.
- **`AURA.Connectors.Outlook`/`AURA.Connectors.Word`** target plain `net8.0` (Graph is just
  HTTP; OpenXML is pure managed code) so they can be unit tested cross-platform too, even
  though they aren't referenced by the Desktop app yet (see "What's implemented").

```powershell
dotnet build AURA.sln
dotnet test tests/AURA.Core.Tests/AURA.Core.Tests.csproj
dotnet test tests/AURA.Host.Tests/AURA.Host.Tests.csproj
dotnet run --project src/AURA.Desktop/AURA.Desktop.csproj   # Windows only
```

> **This code, including the `AURA.Host` project added in this pass, was written and reviewed
> without a .NET toolchain available in the authoring environment** — outbound access to
> NuGet, dot.net, and the package archive is blocked by the sandbox's egress policy (checked
> and re-confirmed blocked more than once), so none of the above commands could actually be run
> here. Every file was written carefully and reviewed line-by-line for correctness (including a
> structural brace/paren balance pass), but this is still **the single highest-risk item in the
> repo** — please run `dotnet build`/`dotnet test` on a real machine or CI before relying on any
> of it, and treat that as blocking further work, not optional.

Once running, the Desktop app shows the tray icon and the robot avatar immediately. Use the
**Control Panel** (tray icon → *Open Control Panel*, or double-click the tray icon) to pick a
scenario (simulated Outlook compose issue or Word document issue) and press **Start** — the
avatar thinks, then surfaces a suggestion bubble with Apply/Ignore, exactly per the Suggest →
Preview → Approve → Verify → Audit flow in the spec's §7.

## Adding a new rule

Rules are plain JSON (README §11) — drop a file in `samples/rules/` (or point
`RuleEngine.LoadRulesFromDirectory` at your own folder) and Core picks it up with no code
change:

```jsonc
{
  "name": "Missing Email Subject",
  "trigger": "email.compose",          // matches AuraContext.Trigger, or "*" for any
  "conditions": [
    { "field": "email.bodyText", "op": "isNotEmpty" },
    { "field": "email.subject",  "op": "isEmpty" }
  ],
  "severity": "warning",
  "message": { "ar": "نسيت عنوان الرسالة", "en": "Subject is empty" },
  "suggestedAction": { "type": "setField", "field": "email.subject" },
  "autonomy": "suggest",               // observe | notify | suggest | askAndAct | autoAct
  "audit": true
}
```

`field` is a dotted path into the camelCase-serialized `AuraContext` (e.g.
`document.styles[0].font`). Supported `op` values: `isEmpty`, `isNotEmpty`, `equals`,
`notEquals`, `contains`, `notContains`, `greaterThan`, `lessThan`, `matchesRegex`, `in`.

## Adding a new Document Standard Profile

Profiles (README §13) replace hard-coded Word font checks. Add a JSON file to
`samples/profiles/`:

```jsonc
{
  "name": "Corporate Memo",
  "roles": {
    "title":   { "font": "Arial", "size": 16, "bold": true },
    "body":    { "font": "Arial", "size": 12, "bold": false }
  },
  "alignment": "Justified",
  "lineSpacing": 1.15
}
```

`Aura.Core.Profiles.DocumentProfileChecker.Check(document, profile)` compares a document's
actual `DocumentStyleRun`s against the profile's roles and returns a 0-1 compliance score plus a
list of issues. A connector (see `WordDocumentConnector`) runs this at capture time and stores
the result on `DocumentContext.ComplianceScore`/`ComplianceIssues`, so a plain rule condition
(`document.complianceScore lessThan 1.0`) can trigger on it — the Rule Engine never needs to
know profiles exist.

## Adding a new connector

Implement `Aura.Core.Connectors.IConnector` (fills a `Context`) and, if the app supports
automated fixes, `Aura.Core.Actions.IActionHandler` (Preview/Apply/Verify) - this part hasn't
changed. What has changed: instead of `new`-ing your connector up inside a UI controller, write
an `Aura.Host.Connectors.IConnectorFactory` for it (see `MockConnectorFactories.cs` for the
two existing examples) and register the factory instance where `App.xaml.cs` builds the
`ConnectorRegistry`. Enable it for a tenant by adding an entry under `connectors` in
`tenant.json` with your factory's `Implementation` key - the Rule Engine, Confidence Engine,
Policy Engine, and `DetectionPipeline` never change.

## Configuring a tenant

`AURA.Desktop` loads `tenant.json` next to the executable at startup (falling back to a bundled
default if the file is missing) via `Aura.Host.Configuration.AuraConfigurationLoader`:

```jsonc
{
  "tenantId": "default",
  "rulesDirectory": "Rules",
  "auditSink": "local-file",
  "auditFilePath": "{localAppData}/AURA/audit.jsonl",
  "sessionTimeLimitMinutes": 30,
  "policies": [
    { "app": "*", "ceiling": "askAndAct" }
  ],
  "connectors": [
    { "implementation": "mock-outlook", "enabled": true },
    { "implementation": "mock-word", "enabled": true }
  ],
  "privacyZones": [
    // { "name": "No body capture", "app": "outlook", "field": "email.bodyText" }
  ]
}
```

`privacyZones` entries wholesale-clear a field (README §10) before PII detection even runs -
`app` is the app id ("outlook"/"word"), `field` is the same dotted-path syntax rules use. This
is deliberately a single file for a single running Agent instance, not a multi-tenant
database - per the architecture review, multi-tenancy belongs in a future server/backend that
this Agent would talk to, not inside the Desktop process itself.

## What's implemented vs. deferred

Implemented, matching the build order in the brief:

- **V0.3 Desktop Agent shell** — tray icon, draggable robot avatar with the §14 hi-fi spec
  (92×64 head, mouse-tracking pupils, blink, mood mouth, bob/walk animation, antenna pulse on
  alert), Robot/Minimal/Professional/Invisible styles, AR/EN + RTL toggle.
- **Context → Rule → Confidence → Policy → Suggestion pipeline**, wired end-to-end against a
  mock connector, with full unit + integration test coverage.
- **Suggest → Preview → Approve → Verify → Audit flow**, with a JSON-lines audit log and an
  in-app viewer.
- **Monitoring scope picker, drag-to-draw region selection, live Privacy Indicator, delete
  captured data** (README §10) — functional but intentionally low-fidelity per the brief's §3
  fidelity note (only the avatar + alert flow needed pixel fidelity).
- **Document Standard Profile checker** (README §13), decoupling Word's compliance check from
  any hard-coded font values.
- **Outlook connector** (Microsoft Graph REST + Azure.Identity) and **Word connector**
  (OpenXML SDK) behind the `IConnector`/`IActionHandler` interfaces — read real drafts/documents,
  though not yet registered in `AURA.Host`'s `ConnectorRegistry` (which still only enables the
  two mock factories by default, per the brief: "wire the pipeline against a mock connector
  before integrating Graph API").
- **`AURA.Host` composition root** — `AuraConfiguration`/`tenant.json`, `ConnectorRegistry` +
  `IConnectorFactory`, and `AuraHost.Build` turning both into a ready `DetectionPipeline`.
  `AssistantController` now only owns UI state and windows.
- **`IAuditSink`** (renamed from `IAuditLog`) with a `DeleteAllAsync` method, so "delete captured
  data" and swapping the storage backend both go through the interface, not a UI-layer file path.
- **`AuraContext`'s `User`/`Session`/`Security`/`Environment`** sections exist as a stable shape
  for future connectors/AI Gateway/identity work to populate.
- **Privacy Zones + PII detection + redaction**, run by `DetectionPipeline` before the Rule
  Engine sees the context (README §10) - a regex-based detector (email/national-ID/credit-card/
  phone) plus a redactor that safely merges overlapping matches, both behind interfaces so a
  stronger detector can replace the regex one later without touching the pipeline.
- **Session-scoped monitoring auto-stop** (README §10) - configurable per tenant via
  `sessionTimeLimitMinutes`, enforced in `AssistantController`.

Deliberately deferred (later roadmap items, §16, or explicitly lower priority in the brief):

- **Live compose-window monitoring.** Graph only sees saved mail (drafts/sent), not keystrokes
  in a still-open Outlook window. That needs an Outlook Add-in (Office.js) feeding the same
  Context object — a natural next step, not yet built.
- **Writing corrected styles back into a .docx** — `WordDocumentConnector` is read-only; it
  scores compliance but doesn't yet implement `IActionHandler` to auto-fix fonts.
- **Real screen understanding** — UI Automation / OCR / Vision LLM fallback (README §4-6,
  roadmap V0.5). The Desktop app captures context from the mock connector, not the live screen.
- **"Teach AURA" no-code Rule Builder UI** — point-at-an-element, label it, describe the rule in
  plain language, compile to JSON. The JSON rule format and loader exist; the authoring UI
  doesn't.
- **Data Classification, Consent management, Secure Storage (encrypted audit log), Secret
  Management, full Permission Model/RBAC** — the narrower parts of Phase 2 that were left out
  (see "Architecture" above for why).
- **AI Gateway (`IAiProvider`/Model Router), any LLM call** — `ConfidenceEngine` stays the
  deterministic heuristic it already was; no AI abstraction exists yet.
- **Real plugin/DLL-loading connector model** — `ConnectorRegistry` today is populated in code
  at startup (two mock factories), not from a manifest or loaded assembly.
- **Admin Portal, Knowledge/RAG packs, Workflow agent, multi-tenant backend, identity/RBAC,
  licensing/deployment** — roadmap V0.6-V1.0, out of scope for this pass.

## A note on the rest of this repository

This repo's initial commit also contains what look like PyInstaller build-cache files (`wyt.pkg`,
`PYZ-00.pyz`, various `*.toc` files) for an unrelated program called `wyt.py`. They weren't
touched as part of this work — flagging only in case they were committed by accident.
