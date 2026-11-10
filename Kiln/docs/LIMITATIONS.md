# Known limitations and unsupported combinations

| Area | Limitation | Reason / alternative |
|---|---|---|
| Drag wheel | The wheel does not appear *during* an Explorer drag (Tangerine/OpenShelf behaviour) | Windows has no supported way to observe modifiers during another app's OLE drag without global low-level hooks. Alternatives: wheel on drop, floating drop zone (Ctrl+Alt+K), Explorer-selection hotkey (Ctrl+Alt+Shift+K), context menu |
| Context menu | On Windows 11 it is under "Show more options" | The top-level menu needs a signed MSIX/sparse package with an IExplorerCommand handler |
| PDF → DOCX | **Not offered** | Word's PDF import blocks on a modal dialog under hidden automation (verified: hangs). Needs LibreOffice or pdf2docx, neither installed |
| HEIC/HEIF output | Not offered; HEIC tools save as JPEG | The standard ImageMagick Windows build decodes HEIC only (`HEIC r--`) |
| SVG output | Not offered | No raster→vector tracing |
| PDF compression | Lossless (qpdf stream optimisation) only; "Already optimized" is reported when there's no gain | Image downsampling needs Ghostscript (not installed) |
| RAR | Extract only | RAR creation is proprietary |
| TAR / TAR.GZ | No password | Format has no encryption; use ZIP (AES-256) or 7z |
| Office formats | Need Microsoft Word/Excel/PowerPoint; ODT/ODS/ODP via Office. Excel → CSV saves the active sheet only | No LibreOffice installed. Office jobs are serialized and have a 5-min watchdog |
| Markdown/EPUB → PDF | Pandoc → DOCX → Word (needs Word) | No LaTeX engine |
| Pause | Pauses the *queue*; running engine processes are not suspended | Suspending encoders mid-write is unsafe |
| ETA | Shown only when progress is measurable (FFmpeg, 7-Zip, per-page PDF work) | Other engines don't report progress |
| 7-Zip passwords | Passed on the 7-Zip command line (visible to other processes of the same user) | 7-Zip has no password-file option. PDF passwords use a temp argument file |
| Drag from browsers | Dropping web images that aren't files is not supported (a notice is shown) | WebView2 only exposes file paths |
| Toolchain | Built with the GNU Rust toolchain on the dev machine (no MSVC C++ workload); one harmless linker warning ("multiple non-default manifests") | Use the MSVC toolchain where available |
| Installer size | All engines are bundled: ~830 MB installed; the installer is a few hundred MB | Needed so a fresh PC works offline. FFmpeg (2 × 217 MB static exes) and Pandoc (228 MB) are most of it |
| Redistribution licenses | The bundle contains GPL tools (FFmpeg full build, Pandoc, Poppler) | Fine for personal use on your own PCs. Distributing the installer publicly requires offering their source code and license texts |
| WebView2 | If a Windows 10 PC lacks the WebView2 runtime, the installer downloads it | Use `webviewInstallMode: offlineInstaller` (+~130 MB) for fully offline installs on such PCs |
| Code signing | Installer and exe are unsigned → SmartScreen warning on first run | Needs a code-signing certificate |
| Windows 10 | Not tested on a Windows 10 machine (dev machine is Windows 11) | Uses only APIs available on Windows 10 1809+; WebView2 bootstrapper is bundled in the installer |
