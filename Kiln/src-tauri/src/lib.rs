//! Kiln desktop shell: Tauri commands, tray, global shortcuts, single instance, drop zone.

mod winint;

use kiln_core::catalog::ActionInfo;
use kiln_core::engines::{EngineId, EngineStatus, Engines};
use kiln_core::exec::{actions_for_paths, cleanup_stale_temp, tesseract_languages, JobSpec};
use kiln_core::history::{History, HistoryEntry};
use kiln_core::jobs::{JobManager, JobSnapshot};
use kiln_core::output::OutputSettings;
use kiln_core::{inspect, FileInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State, WindowEvent};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_opener::OpenerExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// "system" | "light" | "dark"
    pub theme: String,
    pub output: OutputSettings,
    pub max_parallel: usize,
    pub launch_at_startup: bool,
    pub close_to_tray: bool,
    pub radial_on_drop: bool,
    pub shortcut_dropzone: String,
    pub shortcut_selection: String,
    pub engine_paths: HashMap<EngineId, PathBuf>,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            theme: "system".into(),
            output: OutputSettings::default(),
            max_parallel: std::thread::available_parallelism().map(|n| (n.get() / 2).clamp(1, 4)).unwrap_or(2),
            launch_at_startup: false,
            close_to_tray: true,
            radial_on_drop: true,
            shortcut_dropzone: "Ctrl+Alt+K".into(),
            shortcut_selection: "Ctrl+Alt+Shift+K".into(),
            engine_paths: HashMap::new(),
        }
    }
}

struct AppState {
    jobs: JobManager,
    history: Arc<Mutex<History>>,
    settings: Mutex<AppSettings>,
    settings_path: PathBuf,
    office_script: Option<PathBuf>,
    selection_script: Option<PathBuf>,
    engines_dir: Option<PathBuf>,
    pending: Mutex<Vec<PathBuf>>,
}

type Res<T> = Result<T, String>;

/// The `engines` folder shipped next to the app (installer), or the dev staging folder.
fn engines_dir(app: &AppHandle) -> Option<PathBuf> {
    let bundled = app.path().resource_dir().ok().map(|d| d.join("engines"));
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("engines");
    [bundled, Some(dev)].into_iter().flatten().find(|p| p.is_dir()).map(|p| kiln_core::output::strip_verbatim(&std::fs::canonicalize(&p).unwrap_or(p)))
}

fn resource(app: &AppHandle, name: &str) -> Option<PathBuf> {
    let bundled = app.path().resource_dir().ok().map(|d| d.join("resources").join(name));
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("resources").join(name);
    [bundled, Some(dev)].into_iter().flatten().find(|p| p.is_file()).map(|p| kiln_core::output::strip_verbatim(&std::fs::canonicalize(&p).unwrap_or(p)))
}

fn paths_from_args(args: &[String]) -> Vec<PathBuf> {
    args.iter().skip(1).filter(|a| !a.starts_with("--")).map(PathBuf::from).filter(|p| p.exists()).collect()
}

// ------------------------------------------------------------------ commands

#[tauri::command]
fn inspect_files(paths: Vec<PathBuf>) -> Vec<FileInfo> {
    paths.iter().map(|p| inspect(p)).collect()
}

/// Files inside a folder (recursive, capped) for "Add Folder".
#[tauri::command]
fn list_folder(path: PathBuf) -> Res<Vec<PathBuf>> {
    let root = kiln_core::output::validate_input(&path)?;
    let mut out = vec![];
    fn walk(p: &Path, out: &mut Vec<PathBuf>, depth: usize) {
        if depth > 8 || out.len() >= 2000 {
            return;
        }
        if let Ok(rd) = std::fs::read_dir(p) {
            let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
            entries.sort();
            for e in entries {
                if e.is_dir() {
                    walk(&e, out, depth + 1);
                } else if e.is_file() {
                    out.push(e);
                }
            }
        }
    }
    walk(&root, &mut out, 0);
    Ok(out)
}

/// Valid actions for the selection, after inspecting the files themselves. Runs on a worker
/// thread because probing media can take a moment.
#[tauri::command]
async fn get_actions(app: AppHandle, paths: Vec<PathBuf>) -> Res<Vec<ActionInfo>> {
    let engines = app.state::<AppState>().jobs.engines();
    tauri::async_runtime::spawn_blocking(move || actions_for_paths(&paths, &engines)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn submit_job(state: State<AppState>, mut spec: JobSpec, use_settings_output: Option<bool>) -> Res<Vec<uuid::Uuid>> {
    if use_settings_output.unwrap_or(true) {
        spec.output = state.settings.lock().unwrap().output.clone();
    }
    state.jobs.submit(spec)
}

#[tauri::command]
fn cancel_job(state: State<AppState>, id: uuid::Uuid) -> bool {
    state.jobs.cancel(id)
}

#[tauri::command]
fn cancel_all(state: State<AppState>) {
    state.jobs.cancel_all()
}

#[tauri::command]
fn set_paused(app: AppHandle, state: State<AppState>, paused: bool) {
    state.jobs.set_paused(paused);
    let _ = app.emit("queue-paused", paused);
}

#[tauri::command]
fn is_paused(state: State<AppState>) -> bool {
    state.jobs.is_paused()
}

#[tauri::command]
fn list_jobs(state: State<AppState>) -> Vec<JobSnapshot> {
    state.jobs.snapshots()
}

#[tauri::command]
fn clear_finished(state: State<AppState>) {
    state.jobs.clear_finished()
}

#[tauri::command]
fn get_history(state: State<AppState>) -> Vec<HistoryEntry> {
    state.history.lock().unwrap().entries.clone()
}

#[tauri::command]
fn clear_history(state: State<AppState>) -> Res<()> {
    state.history.lock().unwrap().clear().map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_history(state: State<AppState>, id: uuid::Uuid) -> Res<()> {
    state.history.lock().unwrap().remove(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn reconvert(state: State<AppState>, id: uuid::Uuid) -> Res<Vec<uuid::Uuid>> {
    let spec = state.history.lock().unwrap().entries.iter().find(|e| e.id == id).and_then(|e| e.spec.clone()).ok_or("This history entry cannot be repeated")?;
    for p in &spec.inputs {
        if !p.exists() {
            return Err(format!("The original file no longer exists: {}", p.display()));
        }
    }
    let mut spec = spec;
    spec.output = state.settings.lock().unwrap().output.clone();
    state.jobs.submit(spec)
}

#[tauri::command]
fn engine_status(state: State<AppState>) -> Vec<EngineStatus> {
    state.jobs.engines().statuses().to_vec()
}

#[tauri::command]
async fn refresh_engines(app: AppHandle) -> Vec<EngineStatus> {
    let state = app.state::<AppState>();
    let overrides = state.settings.lock().unwrap().engine_paths.clone();
    let e = Arc::new(Engines::discover_with(&overrides, state.office_script.clone(), state.engines_dir.as_deref()));
    state.jobs.set_engines(e.clone());
    e.statuses().to_vec()
}

/// Open a visible console running winget for the engine's fixed package id. winget verifies the
/// installer hash and shows its own license prompt; nothing is installed silently.
#[tauri::command]
fn install_engine(id: EngineId) -> Res<()> {
    let pkg = id.winget_id().ok_or("This engine cannot be installed automatically")?;
    let title = format!("Mostafa-converting app - installing {}", id.label());
    std::process::Command::new("cmd")
        .args(["/c", "start", &title, "cmd", "/k", "winget", "install", "--id", pkg, "-e", "--source", "winget"])
        .spawn()
        .map_err(|e| format!("Could not start winget: {e}"))?;
    Ok(())
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> AppSettings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn context_menu_status() -> bool {
    winint::context_menu_installed()
}

#[tauri::command]
fn set_context_menu(enabled: bool) -> Res<()> {
    winint::set_context_menu(enabled)
}

#[tauri::command]
fn save_settings(app: AppHandle, state: State<AppState>, settings: AppSettings) -> Res<AppSettings> {
    let mut s = settings;
    if let Some(d) = &s.output.directory {
        kiln_core::output::validate_output_dir(d)?;
    }
    s.max_parallel = s.max_parallel.clamp(1, kiln_core::jobs::MAX_WORKERS);
    state.jobs.set_max_parallel(s.max_parallel);
    let autostart = app.autolaunch();
    let currently = autostart.is_enabled().unwrap_or(false);
    if s.launch_at_startup != currently {
        let r = if s.launch_at_startup { autostart.enable() } else { autostart.disable() };
        r.map_err(|e| format!("Could not change startup setting: {e}"))?;
    }
    let old = state.settings.lock().unwrap().clone();
    if old.shortcut_dropzone != s.shortcut_dropzone || old.shortcut_selection != s.shortcut_selection {
        register_shortcuts(&app, &s)?;
    }
    std::fs::create_dir_all(state.settings_path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&state.settings_path, serde_json::to_vec_pretty(&s).unwrap()).map_err(|e| e.to_string())?;
    *state.settings.lock().unwrap() = s.clone();
    let _ = app.emit("settings-changed", &s);
    Ok(s)
}

#[tauri::command]
fn open_path(app: AppHandle, path: PathBuf) -> Res<()> {
    if !path.exists() {
        return Err("The file no longer exists".into());
    }
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn reveal_path(app: AppHandle, path: PathBuf) -> Res<()> {
    if !path.exists() {
        return Err("The file no longer exists".into());
    }
    app.opener().reveal_item_in_dir(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn image_metadata(state: State<AppState>, path: PathBuf) -> Res<Vec<(String, String)>> {
    kiln_core::image_metadata(&state.jobs.engines(), &path)
}

#[tauri::command]
fn ocr_languages(state: State<AppState>) -> Vec<String> {
    tesseract_languages(&state.jobs.engines())
}

#[tauri::command]
fn take_pending_files(state: State<AppState>) -> Vec<PathBuf> {
    std::mem::take(&mut *state.pending.lock().unwrap())
}

#[derive(Serialize)]
struct AppInfo {
    version: String,
    tauri: String,
    data_dir: PathBuf,
    temp_dir: PathBuf,
    exe: PathBuf,
}

#[tauri::command]
fn app_info(app: AppHandle) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        tauri: tauri::VERSION.to_string(),
        data_dir: app.path().app_data_dir().unwrap_or_default(),
        temp_dir: kiln_core::exec::temp_root(),
        exe: std::env::current_exe().unwrap_or_default(),
    }
}

#[tauri::command]
fn toggle_dropzone(app: AppHandle) {
    toggle_dropzone_window(&app, None);
}

#[tauri::command]
fn show_main(app: AppHandle, paths: Option<Vec<PathBuf>>) {
    show_main_window(&app);
    if let Some(p) = paths {
        let _ = app.emit_to("main", "open-files", p);
    }
}

// ------------------------------------------------------------------ windows

fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn toggle_dropzone_window(app: &AppHandle, at: Option<(i32, i32)>) {
    let Some(w) = app.get_webview_window("dropzone") else { return };
    let visible = w.is_visible().unwrap_or(false);
    if visible && at.is_none() {
        let _ = w.hide();
        return;
    }
    let size = w.outer_size().unwrap_or(tauri::PhysicalSize::new(300, 300));
    let pos = match at {
        Some((x, y)) => PhysicalPosition::new(x - size.width as i32 / 2, y - size.height as i32 / 2),
        None => match w.primary_monitor().ok().flatten() {
            Some(m) => {
                let ms = m.size();
                let mp = m.position();
                PhysicalPosition::new(mp.x + ms.width as i32 - size.width as i32 - 40, mp.y + ms.height as i32 - size.height as i32 - 90)
            }
            None => PhysicalPosition::new(100, 100),
        },
    };
    let _ = w.set_position(pos);
    let _ = w.show();
    let _ = w.set_focus();
}

fn register_shortcuts(app: &AppHandle, s: &AppSettings) -> Res<()> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut errors = vec![];
    for (accel, which) in [(&s.shortcut_dropzone, "dropzone"), (&s.shortcut_selection, "selection")] {
        if accel.trim().is_empty() {
            continue;
        }
        let sc: Shortcut = match accel.parse() {
            Ok(sc) => sc,
            Err(e) => {
                errors.push(format!("{accel}: {e}"));
                continue;
            }
        };
        let which = which.to_string();
        if let Err(e) = gs.on_shortcut(sc, move |app, _sc, ev| {
            if ev.state() != ShortcutState::Pressed {
                return;
            }
            if which == "dropzone" {
                toggle_dropzone_window(app, None);
            } else {
                selection_shortcut(app);
            }
        }) {
            errors.push(format!("{accel}: {e} (it may be used by another app)"));
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(format!("Shortcut problem: {}", errors.join("; "))) }
}

/// Global shortcut: take the files selected in the focused Explorer window and open the radial
/// wheel at the mouse pointer.
fn selection_shortcut(app: &AppHandle) {
    let hwnd = winint::foreground_window();
    let cursor = winint::cursor_pos();
    let script = app.state::<AppState>().selection_script.clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let paths = script.map(|s| winint::explorer_selection(&s, hwnd)).unwrap_or_default();
        if paths.is_empty() {
            let _ = app.emit("toast", "Select files in a File Explorer window first, then press the shortcut.");
            show_main_window(&app);
            return;
        }
        toggle_dropzone_window(&app, cursor.or(Some((400, 400))));
        let _ = app.emit_to("dropzone", "wheel-files", paths);
    });
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Open Mostafa-converting app", true, None::<&str>)?;
    let drop = MenuItem::with_id(app, "dropzone", "Show / hide drop zone", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause / resume queue", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &drop, &pause, &sep, &quit])?;
    TrayIconBuilder::with_id("kiln")
        .icon(app.default_window_icon().cloned().unwrap())
        .tooltip("Mostafa-converting app")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id().as_ref() {
            "show" => show_main_window(app),
            "dropzone" => toggle_dropzone_window(app, None),
            "pause" => {
                let st = app.state::<AppState>();
                let p = !st.jobs.is_paused();
                st.jobs.set_paused(p);
                let _ = app.emit("queue-paused", p);
            }
            "quit" => {
                app.state::<AppState>().jobs.shutdown();
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

// ------------------------------------------------------------------ entry point

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let paths = paths_from_args(&argv);
            show_main_window(app);
            if !paths.is_empty() {
                let _ = app.emit_to("main", "open-files", paths);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .setup(|app| {
            let handle = app.handle().clone();
            let data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data)?;
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            let mut settings: AppSettings = std::fs::read(&settings_path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
            if let Some(d) = &settings.output.directory {
                if !d.is_dir() {
                    settings.output.directory = None; // folder vanished: fall back to "beside original"
                }
            }
            settings.launch_at_startup = app.autolaunch().is_enabled().unwrap_or(false);
            let office_script = resource(&handle, "office.ps1");
            let selection_script = resource(&handle, "selection.ps1");
            let stale = cleanup_stale_temp();
            if stale > 0 {
                eprintln!("removed {stale} stale temp folders");
            }
            let engines_dir = engines_dir(&handle);
            let engines = Arc::new(Engines::discover_with(&settings.engine_paths, office_script.clone(), engines_dir.as_deref()));
            let history = Arc::new(Mutex::new(History::load(&data.join("history.json"))));
            let h2 = history.clone();
            let emitter = handle.clone();
            let jobs = JobManager::new(
                engines,
                settings.max_parallel,
                Box::new(move |snap: &JobSnapshot| {
                    let _ = emitter.emit("job", snap);
                    if snap.status != kiln_core::jobs::JobStatus::Queued {
                        h2.lock().unwrap().record(snap);
                    }
                }),
            );
            let args: Vec<String> = std::env::args().collect();
            let hidden = args.iter().any(|a| a == "--hidden");
            let pending = paths_from_args(&args);
            if let Err(e) = register_shortcuts(&handle, &settings) {
                eprintln!("{e}");
            }
            app.manage(AppState { jobs, history, settings: Mutex::new(settings), settings_path, office_script, selection_script, engines_dir, pending: Mutex::new(pending) });
            build_tray(&handle)?;
            if !hidden {
                show_main_window(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                if window.label() == "dropzone" {
                    api.prevent_close();
                    let _ = window.hide();
                    return;
                }
                let st = app.state::<AppState>();
                let to_tray = st.settings.lock().unwrap().close_to_tray;
                if to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    st.jobs.shutdown();
                    app.exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            inspect_files, list_folder, get_actions, submit_job, cancel_job, cancel_all, set_paused, is_paused, list_jobs,
            clear_finished, get_history, clear_history, remove_history, reconvert, engine_status, refresh_engines,
            install_engine, get_settings, save_settings, context_menu_status, set_context_menu, open_path, reveal_path,
            image_metadata, ocr_languages, take_pending_files, app_info, toggle_dropzone, show_main
        ])
        .run(tauri::generate_context!())
        .expect("error while running Mostafa-converting app");
}
