// Typed bridge to the Rust backend. Field names mirror the Rust structs (snake_case).
import { invoke } from "@tauri-apps/api/core";

export type Category = "image" | "pdf" | "audio" | "video" | "document" | "spreadsheet" | "presentation" | "archive";
export type Format = string; // Rust enum variant name, e.g. "Jpg", "SevenZ"

export type Action = { kind: string; to?: Format };

export type Param =
  | "image_quality" | "resize" | "crop" | "rotate" | "dpi" | "page_range" | "page_order" | "pdf_rotate"
  | "password" | "ocr_language" | "audio" | "video" | "trim_range" | "fps" | "gif_width";

export interface ActionInfo {
  action: Action;
  label: string;
  group: "convert" | "tool";
  output_category: Category | null;
  recommended: boolean;
  combines_inputs: boolean;
  produces_folder: boolean;
  engines: string[];
  params: Param[];
}

export interface FileInfo {
  path: string;
  name: string;
  size: number;
  is_dir: boolean;
  format: Format | null;
  format_label: string | null;
  category: Category | null;
  detection: string;
  extension_mismatch: boolean;
  error: string | null;
}

export interface Crop { x: number; y: number; width: number; height: number }

export interface Options {
  quality?: number | null;
  width?: number | null;
  height?: number | null;
  scale_pct?: number | null;
  rotate?: number | null;
  flip_h?: boolean;
  flip_v?: boolean;
  crop?: Crop | null;
  strip_metadata?: boolean;
  dpi?: number | null;
  pages?: string | null;
  password?: string | null;
  ocr_lang?: string | null;
  audio_bitrate_kbps?: number | null;
  sample_rate?: number | null;
  channels?: number | null;
  video_codec?: string | null;
  resolution?: number | null;
  fps?: number | null;
  crf?: number | null;
  gif_width?: number | null;
  start?: number | null;
  end?: number | null;
}

export type Conflict = "rename" | "overwrite" | "skip";
export interface OutputSettings { directory: string | null; template: string; conflict: Conflict }

export interface JobSpec { inputs: string[]; action: Action; options: Options; output?: OutputSettings }

export type JobStatus = "queued" | "running" | "done" | "skipped" | "failed" | "cancelled";
export interface JobSnapshot {
  id: string;
  batch: string;
  label: string;
  inputs: string[];
  status: JobStatus;
  progress: number | null;
  stage: string;
  eta_secs: number | null;
  elapsed_ms: number;
  outputs: string[];
  error: string | null;
  note: string | null;
  created: string;
}

export interface HistoryEntry {
  id: string;
  time: string;
  label: string;
  inputs: string[];
  outputs: string[];
  status: string;
  error: string | null;
  note: string | null;
  elapsed_ms: number;
  spec: JobSpec | null;
}

export interface EngineStatus {
  id: string;
  label: string;
  available: boolean;
  path: string | null;
  version: string | null;
  winget_id: string | null;
  problem: string | null;
}

export interface AppSettings {
  theme: "system" | "light" | "dark";
  output: OutputSettings;
  max_parallel: number;
  launch_at_startup: boolean;
  close_to_tray: boolean;
  radial_on_drop: boolean;
  shortcut_dropzone: string;
  shortcut_selection: string;
  engine_paths: Record<string, string>;
}

export interface AppInfo { version: string; tauri: string; data_dir: string; temp_dir: string; exe: string }

export const api = {
  inspectFiles: (paths: string[]) => invoke<FileInfo[]>("inspect_files", { paths }),
  listFolder: (path: string) => invoke<string[]>("list_folder", { path }),
  getActions: (paths: string[]) => invoke<ActionInfo[]>("get_actions", { paths }),
  submitJob: (spec: JobSpec) => invoke<string[]>("submit_job", { spec, useSettingsOutput: true }),
  cancelJob: (id: string) => invoke<boolean>("cancel_job", { id }),
  cancelAll: () => invoke<void>("cancel_all"),
  setPaused: (paused: boolean) => invoke<void>("set_paused", { paused }),
  isPaused: () => invoke<boolean>("is_paused"),
  listJobs: () => invoke<JobSnapshot[]>("list_jobs"),
  clearFinished: () => invoke<void>("clear_finished"),
  getHistory: () => invoke<HistoryEntry[]>("get_history"),
  clearHistory: () => invoke<void>("clear_history"),
  removeHistory: (id: string) => invoke<void>("remove_history", { id }),
  reconvert: (id: string) => invoke<string[]>("reconvert", { id }),
  engineStatus: () => invoke<EngineStatus[]>("engine_status"),
  refreshEngines: () => invoke<EngineStatus[]>("refresh_engines"),
  installEngine: (id: string) => invoke<void>("install_engine", { id }),
  getSettings: () => invoke<AppSettings>("get_settings"),
  saveSettings: (settings: AppSettings) => invoke<AppSettings>("save_settings", { settings }),
  contextMenuStatus: () => invoke<boolean>("context_menu_status"),
  setContextMenu: (enabled: boolean) => invoke<void>("set_context_menu", { enabled }),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  revealPath: (path: string) => invoke<void>("reveal_path", { path }),
  imageMetadata: (path: string) => invoke<[string, string][]>("image_metadata", { path }),
  ocrLanguages: () => invoke<string[]>("ocr_languages"),
  takePendingFiles: () => invoke<string[]>("take_pending_files"),
  appInfo: () => invoke<AppInfo>("app_info"),
  toggleDropzone: () => invoke<void>("toggle_dropzone"),
  showMain: (paths?: string[]) => invoke<void>("show_main", { paths }),
};
