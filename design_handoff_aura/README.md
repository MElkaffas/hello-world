# Handoff: AURA — AI Desktop Assistant (Enterprise Platform)

> Package for a developer using Claude Code (or any engineering team) to turn the AURA
> design prototype into a **real desktop program** that runs outside the browser.

---

## 1. Overview

**AURA** (AI Unified Remote Assistant) is a desktop AI agent that watches a chosen region of
the user's screen, understands the application/document in that region, detects mistakes and
missing information, and offers contextual help — as suggestions the user can **Fix** or
**Ignore**. A small, draggable robot avatar is the visible interface; it can be turned off in
favor of a minimal notification style.

The commercial goal is **not** a single hard-coded tool but a configurable platform:

```
AURA Platform
   Core Engine  (generic — same for every customer)
   Connectors   (Outlook, Word, Excel, Teams, Browser, ERP/CRM…)
   Rule + Policy Engine (no-code, per customer)
   Knowledge Packs (Telecom / Regulatory / Banking / Legal / HR…)
   Admin Portal (users, rules, policies, audit, analytics)
```

The first vertical to ship is a **Regulatory / Licensing Pack** (IoT, Type Approval, VAS,
Satellite, Spectrum, Connected Vehicles).

---

## 2. About the design files

The files in `prototypes/` are **design references created in HTML** — an interactive prototype
demonstrating the intended look, motion and flow. **They are not production code to ship.**

Your task is to **recreate this experience as a native desktop application** using an
appropriate stack (see §4), applying that stack's own UI patterns. The HTML is the source of
truth for *visual design and interaction*, not for architecture — the architecture is described
in §5–§13 below and deliberately differs from the prototype (the prototype hard-codes rules
that must become a data-driven engine).

### How to open the prototype
- `prototypes/AURA Desktop Assistant.html` — self-contained, opens in any browser, works offline.
- `prototypes/Desktop Assistant.dc.html` — source component (needs the `_ds/` folder + `support.js` alongside it, both included).

---

## 3. Fidelity

**High-fidelity** for the floating assistant + monitoring/alert flow: final colors, type,
spacing, motion and copy (AR + EN). Recreate these pixel-faithfully in the desktop UI layer.

The Admin Portal, Rule Builder, Privacy Center, Knowledge Packs and Analytics screens are
**specified in this README but not yet drawn** — treat their descriptions as low-fidelity
functional requirements and style them with the Industry design system (§14).

---

## 4. Recommended technology

| Layer | Recommendation | Why |
| --- | --- | --- |
| Desktop shell | **C#/.NET (WPF or WinUI 3)** on Windows; Electron if cross-platform is required | WPF gives first-class access to Windows UI Automation |
| Screen understanding | **Windows UI Automation / Accessibility API** first, then **OCR** (Tesseract or Azure AI Vision), then a **Vision LLM** as fallback | UI Automation yields real fields (Subject, To…), not pixels — far more accurate than screenshots |
| App connectors | **Microsoft Graph API** + **Office Add-ins / VSTO** for Outlook & Word | Read/write actual mail & document objects instead of guessing from screen |
| AI | **AI Gateway** abstraction in front of Cloud LLM, on-prem, and local models; a **Model Router** picks the cheapest capable path | Privacy, cost, latency; no lock-in to one provider |
| Knowledge | **RAG** over customer documents, isolated per tenant | Company-specific answers |
| Storage | Per-tenant DB (rules, policies, audit, knowledge index) | Multi-tenant isolation |

**Key architectural correction vs. the prototype:** the prototype does
`if app === 'outlook' { checkSubject() }`. The product must instead evaluate **data-driven
rules** from a Rule Engine against a normalized **context object** produced by connectors +
screen understanding. AURA core must know nothing about Outlook or Word specifically.

---

## 5. System architecture

```
                         AURA PLATFORM
                              │
        ┌─────────────────────┼─────────────────────┐
   Desktop Agent          AI Gateway            Admin Portal
        │                     │                     │
   ┌────┼─────┐         ┌─────┼─────┐         ┌──────┼──────┐
  OCR  UIAuto Connectors LLM Vision Local    Rules Policies Audit
        │                                          │
        └──────────── Context Engine ──────────────┘
                              │
                       Knowledge Engine
                    (Company / Packs / Custom)
```

### The 4 product layers
1. **Core** (fixed for all customers): Desktop Agent, Screen Engine, OCR, UI Automation, AI
   Gateway, Rule Engine, Policy Engine, Security, Audit, Notification.
2. **Connectors**: Outlook, Word, Excel, Teams, Browser, SAP, CRM, ERP, custom apps.
3. **Knowledge Packs**: Telecom, Banking, Legal, HR, Healthcare, Regulatory.
4. **Customer configuration**: rules, policies, templates, workflows, terminology, permissions.

---

## 6. The detection pipeline (replaces hard-coded checks)

```
Capture (scoped region / window / app)
   → Screen Understanding (UI Automation → OCR → Vision)
   → Sensitive Data Detector → Redaction        (privacy, §10)
   → Context Engine builds a normalized Context object
   → Rule Engine evaluates rules against Context
   → Confidence Engine scores each finding        (§9)
   → Policy Engine decides interrupt / suggest / act  (§8)
   → Notification (bubble / minimal card)
   → User: Fix (with Preview + Approval) / Ignore  (§7)
   → Action Executor → Verification
   → Audit Log
```

### Context object (normalized — connectors fill it)
```jsonc
{
  "app": "outlook", "window": "Compose", "scope": "region",
  "email": { "to": [], "cc": [], "subject": "", "bodyText": "...",
             "attachments": [], "mentionsAttachment": true },
  "document": { "styles": [{ "role": "title", "font": "Times New Roman", "size": 22 }] },
  "capturedAt": "ISO-8601", "sensitiveRedacted": true
}
```

---

## 7. Actions must be Suggest → Preview → Approve → Verify

The prototype's **Fix** button mutates state directly. In the product this is unsafe. Required flow:

```
Detected → Suggestion (with preview of the change) → [Apply] [Edit] [Ignore]
        → Action executed → Verification ("✓ Subject added") → Audit entry
```

Never auto-apply unless the customer's policy explicitly enables Auto-Act for that
app + rule (§8).

---

## 8. Policy & autonomy levels

Per application (and per rule), the admin sets an autonomy ceiling:

| Level | Name | Behavior |
| --- | --- | --- |
| 0 | Observe | Watches only |
| 1 | Notify | Tells the user |
| 2 | Suggest | Proposes a fix |
| 3 | Ask & Act | Asks, then performs |
| 4 | Auto-Act | Performs automatically per policy |

Example config:
```
Outlook          Observe✓ Notify✓ Suggest✓ Ask&Act✓ Auto-Act✗
Word Formatting  Observe✓ Notify✓ Suggest✓ Ask&Act✓ Auto-Act✓
Browser          Observe✓ Notify✗ Suggest✗ Ask&Act✗ Auto-Act✗
```

The AI proposes; the **Policy Engine** (with confidence + company policy) decides.

---

## 9. Confidence engine

Every finding carries a confidence score; policy thresholds gate the interruption:
```
> 90%  → Alert
70–90% → Suggest quietly
< 70%  → Do not interrupt
```
Purpose: minimize false positives and "AI nagging".

---

## 10. Privacy (core selling point)

- **Monitoring scope** (extend the prototype's four options): Nothing · Screen Region ·
  Window · Application · Document · UI Element · Workflow · Task.
- **Session-based permissions**: e.g. "Monitor Outlook for 30 min" → auto-stops.
- **Privacy Zones** — never monitored even inside an allowed app: password fields, bank/card,
  private mail, HR data, auth screens.
- **Data classification + redaction** before anything reaches a cloud model
  (`National ID: 298… → [REDACTED]`).
- **Live Privacy Indicator** while monitoring: app, scope, AI processing (Local/Cloud),
  what is captured, plus Pause · Stop · Delete captured data.
- **Deployment**: Cloud · Customer private cloud · On-premise (sensitive bodies keep screen
  content inside their own infrastructure via Local AI).

---

## 11. Rule Engine & "Teach AURA" (no-code)

Rules are **data**, editable by an admin/business user without touching code:
```jsonc
{
  "name": "Missing Email Subject",
  "trigger": "email.compose",
  "conditions": [
    { "field": "email.bodyText", "op": "isNotEmpty" },
    { "field": "email.subject",  "op": "isEmpty" }
  ],
  "severity": "warning",
  "message": { "ar": "نسيت عنوان الرسالة", "en": "Subject is empty" },
  "suggestedAction": { "type": "setField", "field": "email.subject" },
  "autonomy": "suggest",
  "audit": true
}
```
**Teach AURA:** user points at a screen element → labels it ("this is Subject") → states the
rule in plain language → AURA compiles it into the structure above.

---

## 12. Admin Portal (separate from the employee UI)

Sections: Dashboard · Users · Groups · Applications · Rules · Policies · Knowledge ·
Workflows · AI Models · Security · Audit Logs · Analytics · Licenses.

- **Groups** (Licensing / Legal / Finance / HR / IT) each carry their own rules, knowledge,
  apps, permissions, AI model, policies.
- **RBAC roles**: Super Admin · Admin · Manager · Policy Designer · AI Trainer · User · Auditor.
- **Multi-tenant** from day one — Tenant A can never see Tenant B's data.
- **Analytics (ROI)**: Issues Detected / Prevented, Auto Corrections, Suggestions Accepted,
  False-Positive Rate, Estimated Time Saved.
- **Audit log**: date · user · action · before/after · approved-by.

---

## 13. Document Standard Profiles (replaces hard-coded Word rule)

Word formatting checks read from a **profile**, not fixed values:
```
Profile: Corporate Memo
  Title:    Arial / 16 / Bold
  Subtitle: Arial / 14 / Bold
  Body:     Arial / 12 / Regular
  Alignment: Justified   Line spacing: 1.15   Margins: …
```
Customers create profiles (NTRA Memo, Legal Memo, Technical Report…). The engine compares the
document's actual styles to the active profile and reports a compliance score + issues.

---

## 14. Design tokens (Industry design system)

From `_ds/industry-…/styles.css`. Use variables, never raw values.

- **Accent (steel):** `#5980a6` (`--color-accent`), ramp 100–900. Deep steps for text on tint.
- **Ground:** `#f2f2f3` (`--color-bg`). **Text:** `#1d1f20`.
- **Type:** headings **Barlow Condensed** (`--font-heading`), body **Barlow** (`--font-body`).
- **Shape language:** square corners, hairline borders, `+` registration corner marks on cards
  and the primary button (the "blueprint" look). No rounded filled cards.
- **Icons:** Lucide, stroke-width 1.5.
- **Shadows:** `--shadow-sm/md/lg`. **Radius:** 4px scale (`--radius-*`).
- **Focus:** `2px solid var(--color-accent)`, offset 2px.

### Robot avatar (hifi spec)
- Head: 92×64, 2px border `--color-accent-700`, radius 13, fill `--color-neutral-100`.
- Eyes: two 23×25 ellipses `--color-accent-900`; a `#eef6ff` pupil translates toward the mouse
  (angle = atan2 to cursor, distance capped ~4px); blink via `scaleY` keyframe.
- Antenna dot pulses `--color-accent` when an alert is active.
- Mouth changes with mood: neutral bar / alert ring / happy arc.
- Idle motion: `auraBob` (3.4s). Monitoring: `auraWalk` (1.1s). Scan line sweeps the region.
- Assistant styles: **Robot** (full body) · **Minimal** (head only) · Professional (card only,
  no avatar) · Invisible. Prototype implements Robot + Minimal via the `assistantStyle` prop.

---

## 15. Screens in the prototype (recreate these)

**Desktop scene** — general desktop with grid wallpaper hosting two mock app windows:
- *Outlook compose* (520×410): To / Cc / Subject rows, body, optional attachment chip.
- *Word document* (480×470): title, subtitle, paragraph, each tagged with its font/size.
- A **left rail** (258px) with three blueprint panels: Control (brand, EN/AR toggle,
  Start/Stop, Select Area), Monitoring Scope (Off / Region / App / Task + data note),
  Knowledge Base (pack tags).
- A **bottom hint strip** (accent-900 bar) with contextual guidance.

**Interactions implemented:** drag robot anywhere; eyes track mouse; "Select Area" → drag to
draw a monitoring rectangle; the rectangle snaps its label to whichever window it covers;
Start → thinking spinner (~1.6s) → alert bubble(s); Fix mutates the mock field; Ignore
dismisses; multi-alert counter with Next; full AR⇄EN switch (RTL/LTR + translated copy).

**State variables:** `lang`, `robot{x,y}`, `mode(idle|drawing|watching)`, `region`,
`watchedApp`, `scope`, `mood(idle|thinking|alert|happy)`, `alerts[]`, `currentAlert`,
`outlook{to,subject,attached,mentionsAttach}`, `word{fixed}`.

---

## 16. Roadmap (agreed)

```
V0.1  Interactive UX prototype                      ← DONE (this package)
V0.2  Architecture prototype: Settings, Rule Builder, Policy Builder,
      App Profiles, Knowledge Pack, Privacy Center, Action Permission,
      Audit Log, Assistant Modes
V0.3  Real Windows Desktop Agent (leave HTML)
V0.4  Outlook + Word connectors
V0.5  AI Vision / screen understanding
V0.6  Knowledge / RAG
V0.7  Workflow agent
V0.8  Admin Portal
V0.9  Enterprise security (SSO, MFA, tenant isolation, retention)
V1.0  Commercial product (licensing, packs, deployment options)
```

---

## 17. Files in this package
- `prototypes/AURA Desktop Assistant.html` — standalone interactive prototype (open this first).
- `prototypes/Desktop Assistant.dc.html` — component source of the prototype.
- `prototypes/support.js`, `prototypes/_ds/…` — runtime + Industry design system needed by the source.
- `README.md` — this document (self-sufficient; a developer who wasn't in the conversation can build from it).
