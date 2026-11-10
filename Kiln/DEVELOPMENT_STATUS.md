# Development status — Mostafa-converting app 1.0.0 (codename Kiln, 2026-11-06)

## All-in-one installer (latest)
- Renamed to **Mostafa-converting app** (exe `MostafaConverter.exe`, id `app.mostafa.converter`, context menu "Convert with Mostafa-converting app").
- All engines ship inside the installer (`engines/` resource, staged by `scripts/stage-engines.ps1`); the app prefers them over system installs. Installer: **189 MB**, ~830 MB installed.
- Tested with `KILN_BUNDLED_ONLY=1` (system engines and PATH ignored): conversion matrix **all pass**; scenarios **21/21 pass** after two fixes (temp-folder removal retry after cancel; ETA shown after 1 s instead of 2 s).
- Installed-app check (bundled-only mode): installer exit 0, 836 MB installed, context menu registered; MP4 → MP3 through the context-menu path produced a valid 3.0 s MP3 (checked with the bundled ffprobe). The old "Kiln" test install was removed with its own uninstaller (files and context-menu key gone).
- Not testable here: a PC that has *never* had ImageMagick/Tesseract installed. The app sets `MAGICK_HOME`/`TESSDATA_PREFIX` to the bundled folders so they shouldn't need any registry or install state; this is still unconfirmed until it's run on a clean PC.

## Completed and verified
- **Core engine** (`crates/kiln-core`): content detection for 49 formats, capability catalog, file-aware refinement (silent video, scanned PDF), validated options, collision-safe output, temp isolation, output validation (format + decode), job queue with parallelism/pause/cancel/ETA, history with interrupted-job recovery.
- **Engines**: FFmpeg 9.0.2, ImageMagick 7.1.2, Pandoc 3.13, Poppler 25.07, qpdf 12.4.2, 7-Zip 26.05, Tesseract 5.5, MS Word/Excel/PowerPoint (COM). Discovery + version verification + winget install button.
- **Desktop app** (Tauri 2 + React): Home, Workspace (format picker, tools, settings panel), Activity, History, Tools, Settings, About; radial wheel; floating drop zone; tray; global shortcuts; single instance; Explorer context menu; start with Windows; light/dark theme.
- **Installer**: NSIS per-user `Kiln_1.0.0_x64-setup.exe` (1.99 MB); registers/removes context menu and autostart.

## Actual test results
- Rust unit tests: **18/18 pass** (`cargo test -p kiln-core`).
- UI logic tests: **7/7 pass** (`npx vitest run`).
- Real-engine QA (`kiln-qa`, 614 checks: full conversion matrix on a generated dataset + scenarios for batch, cancellation, progress/ETA, collisions, skip policy, beside-original output, naming template, unsupported conversion, invalid options, damaged inputs, wrong password, encrypted ZIP, silent video, wrong extension, mixed selection, folder ZIP, `../` ZIP traversal, interrupted recovery, temp cleanup): third run **609 passed, 5 failed**. The 5 failures (Word → HTML in Windows-1252) were fixed and re-verified with a targeted run (**16/16** HTML checks pass). Final full run (in `QA_REPORT.md`): **613 passed, 1 failed**. The failing check, "Progress percentage and ETA reported", got 10 increasing progress samples but no ETA. The job finished in about 2 s, and ETA is only computed after 2 s of elapsed time, so the test clip is too short for that threshold. Open item: lengthen the scenario's clip or lower the threshold, then re-run.
- Installed-app smoke test on Windows 11 (screenshots taken): installer exit 0, files and registry keys present; app launches (31 MB RAM); `kiln.exe --open file` forwards to the running instance and opens the wheel; pressing **1** produced `Holiday photo.jpg` beside the untouched original, with a history entry and toast; Workspace lists file-specific options; Ctrl+Alt+K shows the drop zone.

## Fixed during QA (root causes)
- PowerShell `$app` variable shadowed the validated `-App` parameter (case-insensitive) → renamed.
- `loudnorm` upsampling to 192 kHz broke Vorbis → explicit output sample rate.
- Audio outputs offered for videos without audio, text extraction for scanned PDFs → file-level inspection.
- Source-archive password was reused for the repacked archive → only used to extract.
- HEIC tools tried to encode HEIC → fall back to JPEG.
- Word PDF import hangs → PDF → DOCX removed; Office watchdog that stops only the job's own Office PID.
- Word HTML in cp1252 → UTF-8 output + legacy-encoding text detection.
- `cdylib` export overflow with MinGW → desktop-only `rlib`.

## Pending / not done
- Release rebuild after the last two cosmetic UI fixes (PDF tile text, `.mpg` label, grid column widths). Source is updated; the installer in `D:\kiln-build\target\release\bundle\nsis` predates them. Run `npx tauri build`.
- Manual check of: tray menu clicks, the Explorer-selection hotkey, drag-and-drop with a real mouse, the uninstaller.
- Windows 10 machine test; code signing.

## Known issues
See [docs/LIMITATIONS.md](docs/LIMITATIONS.md).
