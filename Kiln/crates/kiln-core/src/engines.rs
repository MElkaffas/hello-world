//! Discovery and verification of the external conversion engines.
//!
//! Only the executables enumerated in [`EngineId`] can ever be launched. Their paths come from
//! PATH, well-known install folders, or an explicit user override, and each one is verified by
//! running its version command before it is considered available.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineId {
    Ffmpeg,
    Ffprobe,
    Magick,
    Pandoc,
    Pdftoppm,
    Pdftotext,
    Pdftohtml,
    Pdfinfo,
    Qpdf,
    Sevenzip,
    Tesseract,
    /// Microsoft Office automation (Word/Excel/PowerPoint) through the bundled PowerShell script.
    Word,
    Excel,
    Powerpoint,
}

impl EngineId {
    pub const ALL: &'static [EngineId] = &[
        EngineId::Ffmpeg, EngineId::Ffprobe, EngineId::Magick, EngineId::Pandoc, EngineId::Pdftoppm,
        EngineId::Pdftotext, EngineId::Pdftohtml, EngineId::Pdfinfo, EngineId::Qpdf, EngineId::Sevenzip,
        EngineId::Tesseract, EngineId::Word, EngineId::Excel, EngineId::Powerpoint,
    ];

    pub fn exe(self) -> &'static str {
        match self {
            EngineId::Ffmpeg => "ffmpeg.exe",
            EngineId::Ffprobe => "ffprobe.exe",
            EngineId::Magick => "magick.exe",
            EngineId::Pandoc => "pandoc.exe",
            EngineId::Pdftoppm => "pdftoppm.exe",
            EngineId::Pdftotext => "pdftotext.exe",
            EngineId::Pdftohtml => "pdftohtml.exe",
            EngineId::Pdfinfo => "pdfinfo.exe",
            EngineId::Qpdf => "qpdf.exe",
            EngineId::Sevenzip => "7z.exe",
            EngineId::Tesseract => "tesseract.exe",
            EngineId::Word | EngineId::Excel | EngineId::Powerpoint => "powershell.exe",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            EngineId::Ffmpeg => "FFmpeg",
            EngineId::Ffprobe => "FFprobe",
            EngineId::Magick => "ImageMagick",
            EngineId::Pandoc => "Pandoc",
            EngineId::Pdftoppm => "Poppler pdftoppm",
            EngineId::Pdftotext => "Poppler pdftotext",
            EngineId::Pdftohtml => "Poppler pdftohtml",
            EngineId::Pdfinfo => "Poppler pdfinfo",
            EngineId::Qpdf => "qpdf",
            EngineId::Sevenzip => "7-Zip",
            EngineId::Tesseract => "Tesseract OCR",
            EngineId::Word => "Microsoft Word",
            EngineId::Excel => "Microsoft Excel",
            EngineId::Powerpoint => "Microsoft PowerPoint",
        }
    }

    /// The winget package that provides this engine, used by the "Install" button in Settings.
    /// These ids are fixed in code; nothing user- or network-supplied is ever passed to winget.
    pub fn winget_id(self) -> Option<&'static str> {
        match self {
            EngineId::Ffmpeg | EngineId::Ffprobe => Some("Gyan.FFmpeg"),
            EngineId::Magick => Some("ImageMagick.ImageMagick"),
            EngineId::Pandoc => Some("JohnMacFarlane.Pandoc"),
            EngineId::Pdftoppm | EngineId::Pdftotext | EngineId::Pdftohtml | EngineId::Pdfinfo => Some("oschwartz10612.Poppler"),
            EngineId::Qpdf => Some("QPDF.QPDF"),
            EngineId::Sevenzip => Some("7zip.7zip"),
            EngineId::Tesseract => Some("UB-Mannheim.TesseractOCR"),
            EngineId::Word | EngineId::Excel | EngineId::Powerpoint => None,
        }
    }

    fn version_args(self) -> &'static [&'static str] {
        match self {
            EngineId::Ffmpeg | EngineId::Ffprobe => &["-version"],
            EngineId::Magick | EngineId::Pandoc | EngineId::Qpdf | EngineId::Tesseract => &["--version"],
            EngineId::Pdftoppm | EngineId::Pdftotext | EngineId::Pdftohtml | EngineId::Pdfinfo => &["-v"],
            EngineId::Sevenzip => &[],
            EngineId::Word | EngineId::Excel | EngineId::Powerpoint => &[],
        }
    }

    /// Text that must appear in the version output for the binary to be accepted.
    fn version_marker(self) -> &'static str {
        match self {
            EngineId::Ffmpeg => "ffmpeg version",
            EngineId::Ffprobe => "ffprobe version",
            EngineId::Magick => "ImageMagick",
            EngineId::Pandoc => "pandoc",
            EngineId::Pdftoppm => "pdftoppm version",
            EngineId::Pdftotext => "pdftotext version",
            EngineId::Pdftohtml => "pdftohtml version",
            EngineId::Pdfinfo => "pdfinfo version",
            EngineId::Qpdf => "qpdf version",
            EngineId::Sevenzip => "7-Zip",
            EngineId::Tesseract => "tesseract",
            _ => "",
        }
    }

    fn office_progid(self) -> Option<&'static str> {
        match self {
            EngineId::Word => Some("Word.Application"),
            EngineId::Excel => Some("Excel.Application"),
            EngineId::Powerpoint => Some("PowerPoint.Application"),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineStatus {
    pub id: EngineId,
    pub label: &'static str,
    pub available: bool,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub winget_id: Option<&'static str>,
    pub problem: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Engines {
    found: HashMap<EngineId, (PathBuf, String)>,
    statuses: Vec<EngineStatus>,
    /// Path of the bundled office.ps1 automation script.
    pub office_script: Option<PathBuf>,
    /// Capabilities of the installed ImageMagick (formats it can read/write).
    pub magick_formats: MagickFormats,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MagickFormats {
    pub read: Vec<String>,
    pub write: Vec<String>,
}

impl MagickFormats {
    pub fn can_read(&self, f: &str) -> bool {
        self.read.iter().any(|x| x.eq_ignore_ascii_case(f))
    }
    pub fn can_write(&self, f: &str) -> bool {
        self.write.iter().any(|x| x.eq_ignore_ascii_case(f))
    }
}

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn hidden_command(program: &Path) -> Command {
    let mut c = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

/// Command for an engine executable with the environment it needs. Portable ImageMagick and
/// Tesseract copies locate their configuration/data next to the executable only when told to.
pub fn engine_command(id: EngineId, program: &Path) -> Command {
    let mut c = hidden_command(program);
    if let Some(dir) = program.parent() {
        match id {
            EngineId::Magick => {
                c.env("MAGICK_HOME", dir).env("MAGICK_CONFIGURE_PATH", dir);
                let modules = dir.join("modules");
                if modules.is_dir() {
                    c.env("MAGICK_CODER_MODULE_PATH", modules.join("coders")).env("MAGICK_CODER_FILTER_PATH", modules.join("filters"));
                }
            }
            EngineId::Tesseract => {
                let td = dir.join("tessdata");
                if td.is_dir() {
                    c.env("TESSDATA_PREFIX", td);
                }
            }
            _ => {}
        }
    }
    c
}

/// Folder (inside the app's bundled `engines` directory) that holds an engine.
fn bundled_subdir(id: EngineId) -> &'static str {
    match id {
        EngineId::Ffmpeg | EngineId::Ffprobe => "ffmpeg",
        EngineId::Magick => "imagemagick",
        EngineId::Pandoc => "pandoc",
        EngineId::Pdftoppm | EngineId::Pdftotext | EngineId::Pdftohtml | EngineId::Pdfinfo => "poppler",
        EngineId::Qpdf => "qpdf",
        EngineId::Sevenzip => "7zip",
        EngineId::Tesseract => "tesseract",
        EngineId::Word | EngineId::Excel | EngineId::Powerpoint => "",
    }
}

impl Engines {
    /// Locate and verify every engine. `overrides` maps engine ids to user-chosen executables.
    pub fn discover(overrides: &HashMap<EngineId, PathBuf>, office_script: Option<PathBuf>) -> Engines {
        Self::discover_with(overrides, office_script, None)
    }

    /// Like [`discover`], but engines shipped with the app in `bundled_root` are preferred over
    /// system installs. Setting the environment variable `KILN_BUNDLED_ONLY=1` ignores system
    /// installs entirely (used to test the installer as if on a fresh PC).
    pub fn discover_with(overrides: &HashMap<EngineId, PathBuf>, office_script: Option<PathBuf>, bundled_root: Option<&Path>) -> Engines {
        let bundled_only = std::env::var_os("KILN_BUNDLED_ONLY").map_or(false, |v| v == "1");
        // PowerShell misparses `-File \\?\C:\...` paths, so never pass the verbatim form.
        let office_script = office_script.map(|p| crate::output::strip_verbatim(&p));
        let bundled_root = bundled_root.map(crate::output::strip_verbatim);
        let mut e = Engines { office_script, ..Default::default() };
        for &id in EngineId::ALL {
            let status = if id.office_progid().is_some() {
                e.check_office(id)
            } else {
                let bundled = bundled_root.as_ref().map(|r| r.join(bundled_subdir(id)).join(id.exe()));
                let system = if bundled_only { vec![] } else { candidate_paths(id.exe()) };
                let candidates = overrides.get(&id).cloned().into_iter().chain(bundled).chain(system).collect::<Vec<_>>();
                let mut status = EngineStatus { id, label: id.label(), available: false, path: None, version: None, winget_id: id.winget_id(), problem: Some("Not installed".into()) };
                for p in candidates {
                    if !p.is_file() {
                        continue;
                    }
                    match verify(id, &p) {
                        Ok(v) => {
                            status = EngineStatus { id, label: id.label(), available: true, path: Some(p.clone()), version: Some(v.clone()), winget_id: id.winget_id(), problem: None };
                            e.found.insert(id, (p, v));
                            break;
                        }
                        Err(err) => status.problem = Some(format!("{}: {err}", p.display())),
                    }
                }
                status
            };
            e.statuses.push(status);
        }
        if e.has(EngineId::Magick) {
            e.magick_formats = probe_magick_formats(e.path(EngineId::Magick).unwrap());
        }
        e
    }

    fn check_office(&mut self, id: EngineId) -> EngineStatus {
        let progid = id.office_progid().unwrap();
        let ps = system32().join("WindowsPowerShell").join("v1.0").join("powershell.exe");
        let script_ok = self.office_script.as_ref().map_or(false, |p| p.is_file());
        let registered = progid_registered(progid);
        let available = registered && script_ok && ps.is_file();
        let problem = if !registered {
            Some(format!("{} is not installed", id.label()))
        } else if !script_ok {
            Some("office.ps1 automation script missing".into())
        } else {
            None
        };
        if available {
            self.found.insert(id, (ps.clone(), "COM automation".into()));
        }
        EngineStatus { id, label: id.label(), available, path: available.then_some(ps), version: available.then(|| "COM automation".into()), winget_id: None, problem }
    }

    pub fn has(&self, id: EngineId) -> bool {
        self.found.contains_key(&id)
    }

    pub fn path(&self, id: EngineId) -> Option<&Path> {
        self.found.get(&id).map(|(p, _)| p.as_path())
    }

    pub fn statuses(&self) -> &[EngineStatus] {
        &self.statuses
    }

    pub fn office(&self) -> bool {
        self.has(EngineId::Word) || self.has(EngineId::Excel) || self.has(EngineId::Powerpoint)
    }

    /// Construct a set where the given engines are present, for unit tests of the catalog.
    pub fn fake(ids: &[EngineId]) -> Engines {
        let mut e = Engines::default();
        for &id in ids {
            e.found.insert(id, (PathBuf::from(id.exe()), "test".into()));
        }
        e.magick_formats = MagickFormats {
            read: ["JPEG", "PNG", "WEBP", "HEIC", "AVIF", "TIFF", "BMP", "GIF", "SVG", "ICO", "PDF"].map(String::from).to_vec(),
            write: ["JPEG", "PNG", "WEBP", "HEIC", "AVIF", "TIFF", "BMP", "GIF", "ICO", "PDF"].map(String::from).to_vec(),
        };
        e
    }
}

fn verify(id: EngineId, path: &Path) -> Result<String, String> {
    let out = engine_command(id, path).args(id.version_args()).output().map_err(|e| e.to_string())?;
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let marker = id.version_marker().to_lowercase();
    let line = text.lines().find(|l| l.to_lowercase().contains(&marker)).map(|l| l.trim().to_string());
    match line {
        Some(l) => Ok(l.chars().take(120).collect()),
        None => Err("unexpected version output; not the expected program".into()),
    }
}

fn probe_magick_formats(magick: &Path) -> MagickFormats {
    let mut mf = MagickFormats::default();
    let Ok(out) = engine_command(EngineId::Magick, magick).args(["-list", "format"]).output() else { return mf };
    // Lines look like: "     PNG* PNG       rw-   Portable Network Graphics"
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let name = parts[0].trim_end_matches('*').to_string();
        let mode = parts[2];
        if mode.len() != 3 || !mode.chars().all(|c| matches!(c, 'r' | 'w' | '+' | '-')) {
            continue;
        }
        if mode.starts_with('r') {
            mf.read.push(name.clone());
        }
        if mode.chars().nth(1) == Some('w') {
            mf.write.push(name);
        }
    }
    mf
}

fn system32() -> PathBuf {
    std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows")).join("System32")
}

fn progid_registered(progid: &str) -> bool {
    // HKCR\<ProgID>\CLSID exists when the Office app is registered for automation.
    let out = hidden_command(&system32().join("reg.exe")).args(["query", &format!(r"HKCR\{progid}\CLSID")]).output();
    matches!(out, Ok(o) if o.status.success())
}

/// Possible locations of an executable, most specific first. Includes PATH, winget's link and
/// package folders, and the default install directories of each engine.
pub fn candidate_paths(exe: &str) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            v.push(dir.join(exe));
        }
    }
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let pf = std::env::var_os("ProgramFiles").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));
    if let Some(l) = &local {
        v.push(l.join(r"Microsoft\WinGet\Links").join(exe));
        // winget portable packages: Packages\<id>_<source>\<versioned dir>\...\bin\<exe>
        let pkgs = l.join(r"Microsoft\WinGet\Packages");
        v.extend(find_in_tree(&pkgs, exe, 5));
        v.push(l.join(r"Programs\Tesseract-OCR").join(exe));
        v.push(l.join(r"Pandoc").join(exe));
    }
    for sub in ["7-Zip", "Pandoc", "Tesseract-OCR", "qpdf\\bin"] {
        v.push(pf.join(sub).join(exe));
    }
    // ImageMagick and qpdf install into versioned folders.
    if let Ok(rd) = std::fs::read_dir(&pf) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_lowercase();
            if n.starts_with("imagemagick") || n.starts_with("qpdf") || n.starts_with("poppler") || n.starts_with("ffmpeg") {
                v.push(e.path().join(exe));
                v.push(e.path().join("bin").join(exe));
            }
        }
    }
    v
}

fn find_in_tree(root: &Path, exe: &str, depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if depth == 0 {
        return out;
    }
    let Ok(rd) = std::fs::read_dir(root) else { return out };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(find_in_tree(&p, exe, depth - 1));
        } else if p.file_name().map_or(false, |n| n.eq_ignore_ascii_case(exe)) {
            out.push(p);
        }
    }
    out
}
