# Kiln — Technical documentation

## 1. Research summary (Phase 1)

Both reference apps were studied from their public websites only (no binaries, code or assets were used).

| Aspect | Tangerine (macOS) | OpenShelf (macOS, MIT) | Kiln (Windows) |
|---|---|---|---|
| Invocation | Hold ⌘ while dragging a file → radial wheel; ⌥⌘ for tools | Tap a modifier *during* a drag → wheel (Convert / Tools / Actions); Ctrl+Option+Space on a Finder selection | Drop on the window or the floating drop zone → wheel at the drop point; global shortcut on the Explorer selection; Explorer context menu |
| Wheel contents | Valid targets only; never the file's own format; mixed kinds → notice | Same; mixed selection → intersection | Same rules, enforced by the backend catalog (`catalog.rs`) and re-checked by the executor |
| Output | Beside the original, original never touched | Beside original, `Photo 2.png` on collision | Beside original **or** a chosen folder, template naming, `Photo (2).png` (Windows convention), skip / overwrite policies |
| Progress | not documented | small card with cancel | per-job progress %, stage, ETA, cancel; queue pause; progress ring on the drop zone |
| Engines | built in | optional ffmpeg / LibreOffice / Pandoc installs from Settings | FFmpeg, ImageMagick, Pandoc, Poppler, qpdf, 7-Zip, Tesseract, MS Office (COM); detected + verified; install button runs `winget` visibly |
| Privacy | offline, no account | offline, no telemetry, update check only | offline, no telemetry, no network calls at all |

### Windows feasibility of the "wheel while dragging"

macOS lets an app observe a modifier key during a system drag (global event monitors). On Windows, an
OLE drag-and-drop is a modal loop owned by the *source* application (Explorer). A third-party app can only
participate as a **drop target** of a window it owns; showing UI under the cursor mid-drag would require a
global low-level mouse hook plus an invisible always-on-top target window that captures the drop. Low-level
hooks are fragile (UIPI blocks them against elevated windows, anti-virus heuristics flag them) and would
violate the "never sacrifice stability for a visual effect" rule.

**Implemented alternative (all reliable, no hooks):**
1. Drop files on the main window → the wheel opens exactly at the drop point.
2. **Floating drop zone** (Ctrl+Alt+K, tray menu): a small always-on-top puck; drop on it → wheel opens there; progress ring shows status.
3. **Explorer-selection shortcut** (Ctrl+Alt+Shift+K): reads the selection of the focused Explorer window via the documented `Shell.Application` COM API (read-only script `resources/selection.ps1`) and opens the wheel at the mouse pointer — the closest equivalent of OpenShelf's keyboard wheel.
4. **Explorer context menu**: "Convert with Kiln" / "Compress with Kiln" (per-user `HKCU\Software\Classes` keys, no admin rights). On Windows 11 these appear under *Show more options*: the modern top-level menu requires an `IExplorerCommand` COM server shipped in a signed MSIX/sparse package, which is outside the scope of an unsigned NSIS build.

## 2. Architecture

```
kiln/
├─ crates/kiln-core/           Rust library, no UI dependencies (unit + QA tested)
│  ├─ formats.rs               Format catalog (49 formats, categories, extensions)
│  ├─ detect.rs                Content sniffing (magic bytes, ISO-BMFF brands, EBML, ASF GUIDs,
│  │                           ZIP/OOXML/ODF/EPUB introspection, OLE2 streams, text heuristics)
│  ├─ catalog.rs               Capability catalog: actions valid for inputs × installed engines
│  ├─ engines.rs               Engine discovery, version verification, ImageMagick coder probing
│  ├─ options.rs               Validated conversion settings (ranges, page specs, codecs…)
│  ├─ exec.rs                  Executes actions, file-level refinement, output validation
│  ├─ process.rs               Engine runner: arg vectors, no shell, cancel, progress, stderr capture
│  ├─ output.rs                Naming templates, sanitising, collision-safe reservation, disk space
│  ├─ jobs.rs                  Worker pool, parallelism limit, pause, cancel, throttled events, ETA
│  ├─ history.rs               JSON history with crash ("interrupted") recovery
│  └─ bin/kiln-qa.rs           QA harness: generates dataset, runs full matrix, writes QA_REPORT.md
├─ src-tauri/                  Tauri 2 shell
│  ├─ src/lib.rs               Commands, tray, global shortcuts, single instance, windows, settings
│  ├─ src/winint.rs            Context-menu registry, Explorer selection, cursor/foreground helpers
│  ├─ installer-hooks.nsh      NSIS hooks: register / remove context menu + autostart on uninstall
│  └─ tauri.conf.json          Windows: main + transparent always-on-top drop zone; NSIS per-user
├─ resources/office.ps1        Word/Excel/PowerPoint COM automation (read-only open, SaveAs/Export)
├─ resources/selection.ps1     Prints the selected paths of one Explorer window (read-only)
└─ src/                        React + TypeScript UI (views/, components/, lib/util.ts + tests)
```

### Data flow of one conversion

1. UI calls `get_actions(paths)` → `exec::actions_for_paths`: validate paths → detect formats from content →
   `catalog::actions_for_selection` (intersection for mixed selections) → **file-level refinement**
   (ffprobe stream check removes audio outputs for silent videos, video outputs for audio-only
   containers; `pdftotext` on the first pages removes text extraction from scanned PDFs and recommends OCR).
2. UI submits a `JobSpec {inputs, action, options}`; the backend replaces `output` with saved settings.
3. `JobManager::submit` splits per-file actions into one job per input (batch), queues them.
4. A worker calls `exec::execute`: re-validates options and paths, **re-checks the action against the
   catalog** (the UI cannot request unlisted operations), checks free disk space on the temp and output
   volumes, creates a private temp folder (`%TEMP%\Kiln\<uuid>`), runs engines there.
5. Output validation: exists, non-empty, **format re-detected from content** must match the target, plus a
   decode check (`ffprobe` duration for media, `qpdf --show-npages` for PDF, `magick identify` for images).
   Only then is a final name reserved atomically (`create_new`) and the file moved into place.
6. The temp folder is deleted on every path (RAII). Stale temp folders (>1 h) are purged at start-up.

### Security measures

- Only executables in `EngineId` can run; each is located on known paths and accepted only if its version
  output contains the expected marker. All arguments are vectors (`std::process::Command`), never a shell.
- Every option is validated/clamped (`Options::validate`): page specs by grammar, codecs from a fixed list,
  OCR language `[A-Za-z0-9_+]`, numeric ranges.
- Paths: device namespaces rejected, inputs canonicalised, outputs must be a single normal component under
  the output folder (`ensure_inside`), names sanitised (reserved names, illegal characters, length).
- Sources are protected: they are excluded from overwrite targets, and the QA harness hashes every source
  before/after each run.
- Archive extraction: 7-Zip runs with `-snld` (no dangerous links), extracts into the private temp folder,
  then the tree is checked for reparse points before anything is moved out. A crafted `../escape.txt` ZIP is
  part of the QA suite.
- PDF passwords are passed to qpdf through an argument file (not visible in the process list) that is
  deleted immediately. 7-Zip requires the password on its command line (documented limitation).
- Office automation opens documents read-only with a deliberately wrong password (protected files fail
  instead of prompting), runs serialized, has a 5-minute watchdog, and on cancel/timeout stops only the
  Office process the job started (PID reported by the script) — never the user's own Office windows.

### Toolchain note

The development machine had no MSVC C++ workload and little space on C:, so the project builds with
`stable-x86_64-pc-windows-gnu` (`rust-toolchain.toml`) and MinGW-w64, with build output on `D:`
(`.cargo/config.toml`). On a machine with Visual Studio C++ tools, switch the channel to
`stable-x86_64-pc-windows-msvc` and remove the `target-dir` line.

## 3. Engines and what they do

| Engine | Used for |
|---|---|
| FFmpeg / FFprobe | audio & video conversion, compression, trim, crop, resolution/FPS, merge, normalize (EBU R128), GIF, frames, stream probing, output decode check |
| ImageMagick 7 | raster conversions, quality, resize, crop, rotate/flip, strip metadata, images→PDF, GIF frames, ICO, metadata viewer |
| Poppler | PDF→JPG/PNG/TIFF/WebP (pdftoppm), PDF→TXT, PDF→HTML, page counts |
| qpdf | merge, split, extract/reorder, rotate, lossless optimize, AES-256 protect, unlock |
| Tesseract | OCR of images (TXT or searchable PDF) and scanned PDFs (via pdftoppm + qpdf) |
| Pandoc | DOCX/ODT/HTML/MD/EPUB/RTF ↔ DOCX/ODT/HTML/MD/EPUB/RTF/TXT |
| MS Word / Excel / PowerPoint (COM) | Office formats → PDF and between Office/ODF formats, CSV↔XLSX, slides→images |
| 7-Zip | create ZIP/7z/TAR/TAR.GZ (AES-256 for ZIP/7z), extract ZIP/RAR/7Z/TAR/GZ, archive conversion |

## 4. Building

```
npm install
npm run build                     # frontend → dist/
cargo test -p kiln-core           # unit tests
cargo run -p kiln-core --bin kiln-qa -- D:\kiln-build\qa QA_REPORT.md   # real-engine QA
npx tauri build                   # release exe + NSIS installer
```
