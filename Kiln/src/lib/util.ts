import type { Action, ActionInfo, Category, Options, Param } from "../api";

export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n < 0) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return i === 0 ? `${n} B` : `${v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`;
}

export function formatDuration(secs: number | null | undefined): string {
  if (secs == null || !Number.isFinite(secs)) return "";
  const s = Math.max(0, Math.round(secs));
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ${s % 60}s`;
  return `${Math.floor(m / 60)}h ${m % 60}m`;
}

/** Parse "90", "1:30", "01:02:03.5" into seconds. Empty → null. Invalid → NaN. */
export function parseTime(t: string): number | null {
  const s = t.trim();
  if (!s) return null;
  const parts = s.split(":");
  if (parts.length > 3 || parts.some((p) => p.trim() === "" || !/^\d+(\.\d+)?$/.test(p.trim()))) return NaN;
  return parts.reduce((acc, p) => acc * 60 + parseFloat(p), 0);
}

export function actionKey(a: Action): string {
  return a.to ? `${a.kind}:${a.to}` : a.kind;
}

export function sameAction(a: Action, b: Action): boolean {
  return actionKey(a) === actionKey(b);
}

export const categoryLabels: Record<Category, string> = {
  image: "Images",
  pdf: "PDF",
  audio: "Audio",
  video: "Video",
  document: "Documents",
  spreadsheet: "Spreadsheets",
  presentation: "Presentations",
  archive: "Archives",
};

export const categoryOrder: Category[] = ["image", "pdf", "document", "spreadsheet", "presentation", "audio", "video", "archive"];

/** Group conversion actions by output category, recommended first inside each group. */
export function groupConversions(actions: ActionInfo[], query = ""): { category: Category | "other"; items: ActionInfo[] }[] {
  const q = query.trim().toLowerCase();
  const conv = actions.filter((a) => a.group === "convert" && (!q || a.label.toLowerCase().includes(q) || (a.action.to ?? "").toLowerCase().includes(q)));
  const groups = new Map<Category | "other", ActionInfo[]>();
  for (const a of conv) {
    const k = a.output_category ?? "other";
    if (!groups.has(k)) groups.set(k, []);
    groups.get(k)!.push(a);
  }
  const order: (Category | "other")[] = [...categoryOrder, "other"];
  return order
    .filter((k) => groups.has(k))
    .map((k) => ({ category: k, items: groups.get(k)!.sort((a, b) => Number(b.recommended) - Number(a.recommended)) }));
}

/** Pick up to n actions for the radial wheel: recommended conversions, then others, then tools. */
export function wheelActions(actions: ActionInfo[], n = 8): ActionInfo[] {
  const rec = actions.filter((a) => a.group === "convert" && a.recommended);
  const conv = actions.filter((a) => a.group === "convert" && !a.recommended);
  const tools = actions.filter((a) => a.group === "tool" && a.params.every((p) => !requiredParams.includes(p)) && !a.action.kind.startsWith("archive_create"));
  const out: ActionInfo[] = [];
  for (const a of [...rec, ...conv.slice(0, 4), ...tools]) {
    if (out.length >= n) break;
    if (!out.some((o) => sameAction(o.action, a.action))) out.push(a);
  }
  return out;
}

/** Params that have no sensible default: actions needing them can't run from the wheel. */
export const requiredParams: Param[] = ["crop", "page_order", "password", "trim_range", "pdf_rotate", "rotate", "resize"];

/** Validate the settings the user typed for an action. Returns an error message or null. */
export function validateOptions(info: ActionInfo, o: Options): string | null {
  const k = info.action.kind;
  if (k === "image_resize" && !o.width && !o.height && !o.scale_pct) return "Enter a width, height or scale.";
  if ((k === "image_crop" || k === "video_crop") && !o.crop) return "Enter the crop area.";
  if (o.crop && (o.crop.width <= 0 || o.crop.height <= 0)) return "Crop width and height must be positive.";
  if (k === "image_rotate" && !o.rotate && !o.flip_h && !o.flip_v) return "Choose a rotation or flip.";
  if (k === "pdf_rotate" && !o.rotate) return "Choose a rotation angle.";
  if (k === "pdf_pages" && !o.pages?.trim()) return "Enter the pages to keep, e.g. 3,1-2.";
  if ((k === "pdf_protect" || k === "pdf_unlock") && !o.password) return "Enter a password.";
  if (k === "trim" && o.start == null && o.end == null) return "Enter a start and/or end time.";
  if (k === "video_resize" && !o.resolution && !o.fps) return "Choose a resolution or frame rate.";
  if (o.start != null && Number.isNaN(o.start)) return "Start time is not valid (use seconds or mm:ss).";
  if (o.end != null && Number.isNaN(o.end)) return "End time is not valid (use seconds or mm:ss).";
  if (o.start != null && o.end != null && o.end <= o.start) return "End time must be after the start time.";
  if (o.pages && !/^\s*(z|r?\d+)(\s*-\s*(z|r?\d+))?(\s*,\s*(z|r?\d+)(\s*-\s*(z|r?\d+))?)*\s*$/.test(o.pages)) return "Pages must look like 1-3,5,z.";
  if (o.quality != null && (o.quality < 1 || o.quality > 100)) return "Quality must be 1–100.";
  return null;
}

/** File extension Kiln writes for a backend format id (mirrors Format::primary_ext). */
export function extOf(format: string | undefined): string {
  const special: Record<string, string> = { Mpeg: "mpg", SevenZ: "7z", TarGz: "tar.gz" };
  if (!format) return "";
  return special[format] ?? format.toLowerCase();
}

export function fileName(p: string): string {
  const i = Math.max(p.lastIndexOf("\\"), p.lastIndexOf("/"));
  return i >= 0 ? p.slice(i + 1) : p;
}

export function dirName(p: string): string {
  const i = Math.max(p.lastIndexOf("\\"), p.lastIndexOf("/"));
  return i >= 0 ? p.slice(0, i) : p;
}

/** Preview of the naming template, mirroring the Rust implementation for display. */
export function previewName(template: string, sample: string, ext: string): string {
  const dot = sample.lastIndexOf(".");
  const stem = dot > 0 ? sample.slice(0, dot) : sample;
  const srcExt = dot > 0 ? sample.slice(dot + 1) : "";
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  const t = template.trim() || "{name}";
  const s = t
    .split("{name}").join(stem)
    .split("{ext}").join(srcExt)
    .split("{date}").join(`${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`)
    .split("{time}").join(`${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`)
    .split("{action}").join("");
  return `${s.replace(/[<>:"/\\|?*]/g, "_").trim()}.${ext}`;
}
