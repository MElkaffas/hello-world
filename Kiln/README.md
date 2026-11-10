# Mostafa-converting app (project codename: Kiln)

All-in-one installer: every conversion engine ships inside it, so it installs on any Windows 10/11
x64 PC and works offline. To rebuild: `scripts\stage-engines.ps1`, then `npx tauri build`.

A local file converter and file-tools utility for Windows 10/11 (x64). **Drag → Choose → Convert → Done.**

- Images, PDF, audio, video, Office/ODF/Markdown/EPUB documents, archives
- Content-based type detection and file-aware recommendations
- Radial action wheel on drop, floating drop zone, Explorer context menu, Explorer-selection hotkey, tray
- Batch processing in parallel, progress/ETA, cancel, pause queue, history with re-convert
- Fully offline: no account, no telemetry, no uploads; originals are never modified

Built with Tauri 2 (Rust) + React/TypeScript. Conversions are carried out by verified local engines:
FFmpeg, ImageMagick, Poppler, qpdf, Tesseract, Pandoc, 7-Zip and Microsoft Office (if installed).

| Document | Contents |
|---|---|
| [docs/USER_GUIDE.md](docs/USER_GUIDE.md) | How to use Kiln |
| [docs/TECHNICAL.md](docs/TECHNICAL.md) | Research, architecture, security, engines, build |
| [docs/LIMITATIONS.md](docs/LIMITATIONS.md) | Known limitations and unsupported combinations |
| [QA_REPORT.md](QA_REPORT.md) | Generated report of the real-engine test run |
| [DEVELOPMENT_STATUS.md](DEVELOPMENT_STATUS.md) | Completed / pending features, known issues, test results |

## Quick start (development)

```
npm install
npm run tauri dev
```

Tests:

```
cargo test -p kiln-core                 # Rust unit tests
npx vitest run                          # UI logic tests
cargo run -p kiln-core --bin kiln-qa -- D:\kiln-build\qa QA_REPORT.md
```

Release build + installer: `npx tauri build` → `target/release/bundle/nsis/Kiln_1.0.0_x64-setup.exe`.
