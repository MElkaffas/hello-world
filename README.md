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
                              Connector/ActionHandler interfaces, mock connector, audit log,
                              Document Standard Profile checker. Plain net8.0 - no Windows
                              dependency, builds and tests on any OS.
  AURA.Desktop/                WPF tray app + floating robot avatar + control panel + alert
                              bubble (net8.0-windows). Wired to AURA.Core using the mock
                              connector by default.
  AURA.Connectors.Outlook/     Reads/writes Outlook drafts via Microsoft Graph REST + Azure.Identity.
  AURA.Connectors.Word/        Reads a .docx's real styles via the OpenXML SDK and scores them
                              against a Document Standard Profile.
tests/
  AURA.Core.Tests/             xunit unit + integration tests for the Core pipeline.
samples/
  rules/                       Example JSON rules (README §11 schema).
  profiles/                    Example Document Standard Profile JSON (README §13 schema).
design_handoff_aura/           The original design handoff package (spec + HTML prototype),
                              copied in verbatim as the source of truth for visual design.
```

## Build & run

- **`AURA.Core` and `AURA.Core.Tests`** target plain `net8.0` and have no Windows dependency —
  they build and `dotnet test` on Linux/macOS/Windows/CI.
- **`AURA.Desktop`** targets `net8.0-windows` (WPF + WinForms for the tray icon) and can only be
  built/run on Windows with the `Microsoft.NET.Sdk.WindowsDesktop` workload.
- **`AURA.Connectors.Outlook`/`AURA.Connectors.Word`** target plain `net8.0` (Graph is just
  HTTP; OpenXML is pure managed code) so they can be unit tested cross-platform too, even
  though the Desktop app that consumes them is Windows-only.

```powershell
dotnet build AURA.sln
dotnet test tests/AURA.Core.Tests/AURA.Core.Tests.csproj
dotnet run --project src/AURA.Desktop/AURA.Desktop.csproj   # Windows only
```

> **This code was written and reviewed without a .NET toolchain available in the authoring
> environment** — outbound access to NuGet, dot.net, and the package archive was blocked by
> the sandbox's egress policy, so none of the above commands could actually be run here. The
> code was written carefully and reviewed line-by-line for correctness, but please run
> `dotnet build`/`dotnet test` yourself before relying on it, and open an issue (or just fix it
> — it's a normal C# repo) if something doesn't compile.

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
automated fixes, `Aura.Core.Actions.IActionHandler` (Preview/Apply/Verify). Register an instance
with `DetectionPipeline` and the rest of the pipeline (Rule → Confidence → Policy → Suggestion →
Audit) works unchanged — that's the whole point of the interface boundary.

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
  though not yet wired into the Desktop app's default startup path (which still uses the mock
  connector, per the brief: "wire the pipeline against a mock connector before integrating
  Graph API").

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
- **Admin Portal, Knowledge/RAG packs, Workflow agent, multi-tenant RBAC, licensing/deployment**
  — roadmap V0.6-V1.0, out of scope for this pass.

## A note on the rest of this repository

This repo's initial commit also contains what look like PyInstaller build-cache files (`wyt.pkg`,
`PYZ-00.pyz`, various `*.toc` files) for an unrelated program called `wyt.py`. They weren't
touched as part of this work — flagging only in case they were committed by accident.
