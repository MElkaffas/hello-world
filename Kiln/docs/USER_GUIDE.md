# Kiln — User guide

Kiln converts and processes files on your PC: images, PDFs, audio, video, documents and archives.
Nothing is uploaded and no account is needed.

## Install

> The app was renamed to **Mostafa-converting app**; "Kiln" below is the project's internal name.

Copy `Mostafa-converting app_1.0.0_x64-setup.exe` to any Windows 10/11 (64-bit) PC, for example on a
USB stick, and run it. It installs for your Windows user only (no administrator rights), adds a
Start-menu entry, and registers **Convert with Mostafa-converting app** in the Explorer right-click menu.
Uninstall from *Settings → Apps*; this also removes the context-menu entries and the start-with-Windows
entry.

The installer is all-in-one: FFmpeg, ImageMagick, Pandoc, Poppler, qpdf, 7-Zip and Tesseract ship inside
it, so nothing else needs installing and no internet is needed. Exceptions:
- **Word/Excel/PowerPoint conversions** need Microsoft Office on that PC; without it those options are hidden.
- **WebView2**: Windows 11 has it. On an old Windows 10 without it, the installer downloads it (internet needed once).
- **SmartScreen** shows "Windows protected your PC" because the installer isn't code-signed: click
  *More info* → *Run anyway*.

## Convert in three moves

1. **Drag** files (or folders) from Explorer onto the Kiln window.
2. A **wheel** opens where you dropped. Recommended targets are marked. Click a wedge, press its number
   (1–8), or use the arrow keys and Enter. **Tab** switches between *Convert* and *Tools*. Click the
   centre ("More…") for every option and its settings. **Esc** closes the wheel.
3. Kiln saves the result next to the original (for example `Holiday.heic` → `Holiday.jpg`). The original
   is never changed or replaced. If `Holiday.jpg` already exists, you get `Holiday (2).jpg`.

Other ways to start:
- **Right-click** files in Explorer → *Convert with Kiln* (Windows 11: *Show more options* first).
  Right-click a folder → *Compress with Kiln*.
- **Floating drop zone:** press **Ctrl+Alt+K** (or use the tray icon). Drop files on the round puck; the
  wheel opens there and the puck shows progress. Click it afterwards to show the result. Drag it to move it;
  Esc or × hides it.
- **Explorer selection:** select files in Explorer and press **Ctrl+Alt+Shift+K**. The wheel opens at the
  mouse pointer.
- **Add files / Add folder** buttons, or **Ctrl+O** / **Ctrl+Shift+O**.

## Workspace (all options)

The left side lists the added files with their **real** type (Kiln reads file contents; a PNG named
`.jpg` is shown as "really PNG"). The right side shows:
- **Convert to:** target formats grouped by kind, recommended ones starred, with search.
- **Tools:** compress, resize, crop, rotate, remove metadata, merge, split, OCR, trim…
  Only tools that work for all selected files are shown.

Pick an option, adjust its settings (quality, size, pages, bitrate, codec, trim times…), then click
**Convert** (Ctrl+Enter). Several files are processed in parallel.

## Activity, history

- **Activity** shows each job with progress, time remaining (when it can be estimated), **Cancel**, and
  **Open / Show** when done. **Pause queue** holds jobs that haven't started yet.
- **History** keeps past conversions: search, filter, show the output in Explorer, **convert again**
  with the same settings, or clear the list (your files are not deleted). If Kiln was closed during a
  conversion, that entry is marked *interrupted*; nothing partial was written to your folder.

## Settings

- **Output:** next to the original, or always into one folder. File-name template with `{name}`, `{ext}`,
  `{date}`, `{time}`. What to do when a name exists: keep both (default), skip, or replace. Kiln asks
  before turning on replace, and your source files are protected either way.
- **Behaviour:** theme, how many conversions run at once, the wheel on drop, keep running in the tray,
  start with Windows, the Explorer context menu.
- **Global shortcuts:** change or clear them.

## Tips

- PDF that is a scan? Use **OCR to searchable PDF**. Kiln hides "extract text" when a PDF has no text layer.
- Compressing something that is already small? Kiln reports *Already optimized* instead of writing a
  bigger file.
- Password-protected PDF: **Remove password** only works when you enter the correct password; use it only
  for files you are authorized to unlock.
