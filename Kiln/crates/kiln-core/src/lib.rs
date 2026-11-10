//! Kiln core: everything that does not depend on the desktop shell.
//!
//! * [`detect`]   – content-based file type detection
//! * [`catalog`]  – which actions are valid for which inputs with the installed engines
//! * [`engines`]  – discovery and verification of FFmpeg, ImageMagick, Pandoc, …
//! * [`exec`]     – carrying out an action, validating and placing the output
//! * [`jobs`]     – background queue with parallelism, pause and cancellation
//! * [`history`]  – persisted conversion history

pub mod catalog;
pub mod detect;
pub mod engines;
pub mod exec;
pub mod formats;
pub mod history;
pub mod jobs;
pub mod options;
pub mod output;
pub mod process;

use serde::Serialize;
use std::path::{Path, PathBuf};

/// Information shown for an added file.
#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub format: Option<formats::Format>,
    pub format_label: Option<&'static str>,
    pub category: Option<formats::Category>,
    pub detection: &'static str,
    pub extension_mismatch: bool,
    pub error: Option<String>,
}

pub fn inspect(path: &Path) -> FileInfo {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.display().to_string());
    let base = FileInfo { path: path.to_path_buf(), name, size: 0, is_dir: false, format: None, format_label: None, category: None, detection: "unknown", extension_mismatch: false, error: None };
    let canon = match output::validate_input(path) {
        Ok(p) => p,
        Err(e) => return FileInfo { error: Some(e), ..base },
    };
    if canon.is_dir() {
        return FileInfo { path: canon.clone(), is_dir: true, size: output::dir_size(&canon), detection: "folder", ..base };
    }
    let size = std::fs::metadata(&canon).map(|m| m.len()).unwrap_or(0);
    match detect::detect_path(&canon) {
        Ok(d) => FileInfo {
            path: canon,
            size,
            format: d.format,
            format_label: d.format.map(|f| f.label()),
            category: d.format.map(|f| f.category()),
            detection: d.method,
            extension_mismatch: d.extension_mismatch,
            error: if d.format.is_none() { Some(if size == 0 { "The file is empty".into() } else { "Unrecognized or unsupported file type".into() }) } else { None },
            ..base
        },
        Err(e) => FileInfo { path: canon, size, error: Some(e.to_string()), ..base },
    }
}

/// Image metadata via ImageMagick (EXIF and basic properties).
pub fn image_metadata(e: &engines::Engines, path: &Path) -> Result<Vec<(String, String)>, String> {
    let p = output::validate_input(path)?;
    let mut arg = p.as_os_str().to_os_string();
    arg.push("[0]");
    let out = process::run(e, engines::EngineId::Magick, &[process::os("identify"), process::os("-format"), process::os("Format=%m\nWidth=%w\nHeight=%h\nColorspace=%[colorspace]\nDepth=%[bit-depth]\nResolution=%x x %y\n%[EXIF:*]%[IPTC:*]"), arg], None, &process::CancelToken::default(), |_| {})
        .map_err(|e| e.to_string())?;
    Ok(out
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim_start_matches("exif:").to_string(), v.trim().to_string()))
        .filter(|(_, v)| !v.is_empty())
        .collect())
}
