//! Windows shell integration: Explorer context menu (per-user registry keys), reading the
//! current Explorer selection, and cursor/foreground-window helpers.

use std::path::{Path, PathBuf};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

const FILE_KEY: &str = r"Software\Classes\*\shell\MostafaConverter";
const DIR_KEY: &str = r"Software\Classes\Directory\shell\MostafaConverter";

pub fn context_menu_installed() -> bool {
    RegKey::predef(HKEY_CURRENT_USER).open_subkey(FILE_KEY).is_ok()
}

pub fn set_context_menu(enabled: bool) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if !enabled {
        for k in [FILE_KEY, DIR_KEY] {
            match hkcu.delete_subkey_all(k) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_string_lossy();
    for (key, label) in [(FILE_KEY, "Convert with Mostafa-converting app"), (DIR_KEY, "Compress with Mostafa-converting app")] {
        let (k, _) = hkcu.create_subkey(key).map_err(|e| e.to_string())?;
        k.set_value("MUIVerb", &label).map_err(|e| e.to_string())?;
        k.set_value("Icon", &format!("\"{exe}\",0")).map_err(|e| e.to_string())?;
        k.set_value("MultiSelectModel", &"Player").map_err(|e| e.to_string())?;
        let (c, _) = hkcu.create_subkey(format!(r"{key}\command")).map_err(|e| e.to_string())?;
        c.set_value("", &format!("\"{exe}\" --open \"%1\"")).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[link(name = "user32")]
extern "system" {
    fn GetForegroundWindow() -> isize;
    fn GetCursorPos(p: *mut [i32; 2]) -> i32;
}

pub fn foreground_window() -> isize {
    unsafe { GetForegroundWindow() }
}

pub fn cursor_pos() -> Option<(i32, i32)> {
    let mut p = [0i32; 2];
    (unsafe { GetCursorPos(&mut p) } != 0).then_some((p[0], p[1]))
}

/// Paths selected in the Explorer window `hwnd`, via the bundled read-only selection.ps1.
pub fn explorer_selection(script: &Path, hwnd: isize) -> Vec<PathBuf> {
    let ps = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows")).join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let mut cmd = std::process::Command::new(ps);
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(kiln_core::engines::CREATE_NO_WINDOW);
    }
    let out = cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"]).arg(script).arg("-Hwnd").arg(hwnd.to_string()).output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).lines().map(str::trim).filter(|l| !l.is_empty()).map(PathBuf::from).filter(|p| p.exists()).collect(),
        Err(_) => vec![],
    }
}
