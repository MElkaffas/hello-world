//! Output naming, collision handling, path validation and disk-space checks.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Conflict {
    /// Pick a free name: "Photo (2).png"
    #[default]
    Rename,
    /// Replace the existing file. Only used after the user explicitly enabled it.
    Overwrite,
    /// Leave the existing file and report the job as skipped.
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputSettings {
    /// None = save beside the original file.
    pub directory: Option<PathBuf>,
    /// Tokens: {name} {ext} {date} {time} {action}. Default "{name}".
    pub template: String,
    pub conflict: Conflict,
}

impl Default for OutputSettings {
    fn default() -> Self {
        OutputSettings { directory: None, template: "{name}".into(), conflict: Conflict::Rename }
    }
}

const RESERVED: &[&str] = &["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"];

/// Make a string safe to use as a single Windows file-name component.
pub fn sanitize_component(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control() { '_' } else { c })
        .collect();
    // No trailing dots/spaces on Windows.
    while out.ends_with('.') || out.ends_with(' ') {
        out.pop();
    }
    let trimmed = out.trim_start().to_string();
    out = if trimmed.is_empty() || trimmed.chars().all(|c| c == '.') { "file".into() } else { trimmed };
    let stem = out.split('.').next().unwrap_or("").to_ascii_uppercase();
    if RESERVED.contains(&stem.as_str()) {
        out = format!("_{out}");
    }
    // Keep names well below MAX_PATH component limit (255).
    if out.chars().count() > 200 {
        out = out.chars().take(200).collect();
    }
    out
}

/// Render the naming template for an input file.
pub fn render_name(template: &str, input: &Path, action_suffix: &str) -> String {
    let stem = file_stem(input);
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("");
    let now = chrono::Local::now();
    let t = if template.trim().is_empty() { "{name}" } else { template };
    let s = t
        .replace("{name}", &stem)
        .replace("{ext}", ext)
        .replace("{date}", &now.format("%Y-%m-%d").to_string())
        .replace("{time}", &now.format("%H%M%S").to_string())
        .replace("{action}", action_suffix.trim());
    let s = if !action_suffix.is_empty() && !t.contains("{action}") { format!("{s} {action_suffix}") } else { s };
    sanitize_component(s.trim())
}

/// File stem that also strips double extensions such as ".tar.gz".
pub fn file_stem(p: &Path) -> String {
    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let lower = name.to_lowercase();
    for double in [".tar.gz", ".tar.bz2", ".tar.xz"] {
        if lower.ends_with(double) && name.len() > double.len() {
            return name[..name.len() - double.len()].to_string();
        }
    }
    p.file_stem().and_then(|s| s.to_str()).unwrap_or("file").to_string()
}

/// Choose the final path for `name` + `ext` inside `dir`. With `Conflict::Rename` the name is
/// reserved atomically (the file is created empty) so concurrent jobs never pick the same one.
pub fn reserve_path(dir: &Path, name: &str, ext: &str, conflict: Conflict, protected: &[PathBuf]) -> std::io::Result<Option<PathBuf>> {
    let make = |n: &str| if ext.is_empty() { dir.join(n) } else { dir.join(format!("{n}.{ext}")) };
    let is_protected = |p: &Path| protected.iter().any(|q| same_path(p, q));
    let first = make(name);
    ensure_inside(dir, &first)?;
    match conflict {
        Conflict::Overwrite if !is_protected(&first) => return Ok(Some(first)),
        Conflict::Skip if first.exists() || is_protected(&first) => return Ok(None),
        _ => {}
    }
    for i in 1..10_000 {
        let candidate = if i == 1 { first.clone() } else { make(&format!("{name} ({i})")) };
        if is_protected(&candidate) {
            continue;
        }
        match fs::OpenOptions::new().write(true).create_new(true).open(&candidate) {
            Ok(_) => return Ok(Some(candidate)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::Other, "Could not find a free file name"))
}

/// Same as [`reserve_path`] for a folder output.
pub fn reserve_dir(dir: &Path, name: &str) -> std::io::Result<PathBuf> {
    for i in 1..10_000 {
        let candidate = if i == 1 { dir.join(name) } else { dir.join(format!("{name} ({i})")) };
        ensure_inside(dir, &candidate)?;
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::Other, "Could not find a free folder name"))
}

fn ensure_inside(dir: &Path, p: &Path) -> std::io::Result<()> {
    let rel = p.strip_prefix(dir).map_err(|_| traversal())?;
    if rel.components().count() != 1 || !matches!(rel.components().next(), Some(Component::Normal(_))) {
        return Err(traversal());
    }
    Ok(())
}

fn traversal() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, "Output path escapes the output folder")
}

pub fn same_path(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy()),
    }
}

/// Validate a user-supplied input path: must exist, be a regular file or folder, and not be a
/// device namespace path.
pub fn validate_input(p: &Path) -> Result<PathBuf, String> {
    let s = p.to_string_lossy();
    if s.starts_with(r"\\.\") || s.starts_with(r"\\?\GLOBALROOT") || s.contains('\0') {
        return Err(format!("Unsupported path: {s}"));
    }
    let meta = fs::metadata(p).map_err(|e| format!("Cannot open {}: {e}", p.display()))?;
    if !(meta.is_file() || meta.is_dir()) {
        return Err(format!("{} is not a regular file or folder", p.display()));
    }
    let canon = fs::canonicalize(p).map_err(|e| format!("Cannot resolve {}: {e}", p.display()))?;
    Ok(strip_verbatim(&canon))
}

/// `canonicalize` returns `\\?\C:\...`; most engines cannot handle that prefix.
pub fn strip_verbatim(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        p.to_path_buf()
    }
}

/// Validate an output directory.
pub fn validate_output_dir(p: &Path) -> Result<PathBuf, String> {
    let meta = fs::metadata(p).map_err(|e| format!("Output folder {} is not available: {e}", p.display()))?;
    if !meta.is_dir() {
        return Err(format!("{} is not a folder", p.display()));
    }
    if meta.permissions().readonly() {
        // On Windows the read-only bit on folders is advisory; probe with a real write.
    }
    let probe = p.join(format!(".kiln-write-test-{}", uuid::Uuid::new_v4()));
    fs::write(&probe, b"").map_err(|e| format!("Cannot write to {}: {e}", p.display()))?;
    let _ = fs::remove_file(probe);
    Ok(strip_verbatim(&fs::canonicalize(p).map_err(|e| e.to_string())?))
}

/// Free bytes on the volume holding `p`.
#[cfg(windows)]
pub fn free_space(p: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(dir: *const u16, avail: *mut u64, total: *mut u64, free: *mut u64) -> i32;
    }
    let wide: Vec<u16> = p.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let (mut a, mut t, mut f) = (0u64, 0u64, 0u64);
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut a, &mut t, &mut f) };
    (ok != 0).then_some(a)
}

#[cfg(not(windows))]
pub fn free_space(_p: &Path) -> Option<u64> {
    None
}

pub fn human_size(n: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < units.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 { format!("{n} B") } else { format!("{v:.1} {}", units[i]) }
}

pub fn dir_size(p: &Path) -> u64 {
    if p.is_file() {
        return fs::metadata(p).map(|m| m.len()).unwrap_or(0);
    }
    let mut total = 0;
    if let Ok(rd) = fs::read_dir(p) {
        for e in rd.flatten() {
            total += dir_size(&e.path());
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(format!("kiln-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn sanitizes_names() {
        assert_eq!(sanitize_component("a/b\\c:d*e?"), "a_b_c_d_e_");
        assert_eq!(sanitize_component("CON"), "_CON");
        assert_eq!(sanitize_component("con.txt"), "_con.txt");
        assert_eq!(sanitize_component("name. . "), "name");
        assert_eq!(sanitize_component(".."), "file");
        assert_eq!(sanitize_component("日本語 фото ✓"), "日本語 фото ✓");
    }

    #[test]
    fn template_rendering() {
        let p = Path::new(r"C:\x\Holiday Photo.heic");
        assert_eq!(render_name("{name}", p, ""), "Holiday Photo");
        assert_eq!(render_name("{name}", p, "compressed"), "Holiday Photo compressed");
        assert_eq!(render_name("{name}-{ext}", p, ""), "Holiday Photo-heic");
        assert_eq!(render_name("../../{name}", p, ""), ".._.._Holiday Photo");
        assert_eq!(file_stem(Path::new("a.b.tar.gz")), "a.b");
    }

    #[test]
    fn collisions_get_numbered_and_never_hit_protected_files() {
        let d = tmp();
        fs::write(d.join("photo.png"), b"x").unwrap();
        let p = reserve_path(&d, "photo", "png", Conflict::Rename, &[]).unwrap().unwrap();
        assert_eq!(p.file_name().unwrap(), "photo (2).png");
        let p3 = reserve_path(&d, "photo", "png", Conflict::Rename, &[]).unwrap().unwrap();
        assert_eq!(p3.file_name().unwrap(), "photo (3).png");
        // Skip leaves existing files alone.
        assert!(reserve_path(&d, "photo", "png", Conflict::Skip, &[]).unwrap().is_none());
        // Overwrite never targets a protected (source) file.
        let src = d.join("photo.png");
        let o = reserve_path(&d, "photo", "png", Conflict::Overwrite, &[src.clone()]).unwrap().unwrap();
        assert_ne!(o, src);
        assert_eq!(fs::read(&src).unwrap(), b"x");
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn rejects_traversal() {
        let d = tmp();
        assert!(reserve_path(&d, "..\\evil", "txt", Conflict::Rename, &[]).is_err());
        assert!(reserve_dir(&d, "../x").is_err());
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn input_validation() {
        let d = tmp();
        let f = d.join("ünïcödé 文件 (1).txt");
        fs::write(&f, b"hi").unwrap();
        assert!(validate_input(&f).is_ok());
        assert!(validate_input(&d.join("missing.txt")).is_err());
        assert!(validate_input(Path::new(r"\\.\PhysicalDrive0")).is_err());
        assert!(!validate_input(&f).unwrap().to_string_lossy().starts_with(r"\\?\"));
        fs::remove_dir_all(d).unwrap();
    }
}
