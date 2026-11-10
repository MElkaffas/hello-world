import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type Action, type ActionInfo, type AppSettings, type EngineStatus, type FileInfo, type HistoryEntry, type JobSnapshot, type Options } from "./api";

export type View = "home" | "workspace" | "jobs" | "history" | "tools" | "settings" | "about";
export interface Toast { id: number; kind: "info" | "ok" | "err"; text: string; action?: { label: string; run: () => void } }

interface Store {
  view: View;
  go: (v: View) => void;
  settings: AppSettings | null;
  saveSettings: (s: AppSettings) => Promise<void>;
  engines: EngineStatus[];
  refreshEngines: () => Promise<void>;
  files: FileInfo[];
  addPaths: (paths: string[]) => Promise<FileInfo[]>;
  removeFile: (path: string) => void;
  clearFiles: () => void;
  preselect: Action | null;
  setPreselect: (a: Action | null) => void;
  jobs: JobSnapshot[];
  paused: boolean;
  history: HistoryEntry[];
  reloadHistory: () => Promise<void>;
  run: (paths: string[], info: ActionInfo | Action, options?: Options) => Promise<boolean>;
  toasts: Toast[];
  toast: (kind: Toast["kind"], text: string, action?: Toast["action"]) => void;
  dismiss: (id: number) => void;
}

const Ctx = createContext<Store>(null as unknown as Store);
export const useStore = () => useContext(Ctx);

export function errText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e && typeof e === "object" && "message" in e) return String((e as Error).message);
  return String(e);
}

export function StoreProvider({ children }: { children: React.ReactNode }) {
  const [view, setView] = useState<View>("home");
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [engines, setEngines] = useState<EngineStatus[]>([]);
  const [files, setFiles] = useState<FileInfo[]>([]);
  const [preselect, setPreselect] = useState<Action | null>(null);
  const [jobMap, setJobMap] = useState<Map<string, JobSnapshot>>(new Map());
  const [paused, setPaused] = useState(false);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const toastId = useRef(1);

  const toast = useCallback((kind: Toast["kind"], text: string, action?: Toast["action"]) => {
    const id = toastId.current++;
    setToasts((t) => [...t.slice(-3), { id, kind, text, action }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), kind === "err" ? 9000 : 5000);
  }, []);
  const dismiss = useCallback((id: number) => setToasts((t) => t.filter((x) => x.id !== id)), []);

  const reloadHistory = useCallback(async () => setHistory(await api.getHistory()), []);
  const refreshEngines = useCallback(async () => setEngines(await api.refreshEngines()), []);

  useEffect(() => {
    api.getSettings().then(setSettings).catch(() => {});
    api.engineStatus().then(setEngines).catch(() => {});
    api.listJobs().then((js) => setJobMap(new Map(js.map((j) => [j.id, j])))).catch(() => {});
    api.isPaused().then(setPaused).catch(() => {});
    reloadHistory().catch(() => {});
    const un: Promise<() => void>[] = [];
    un.push(
      listen<JobSnapshot>("job", (e) => {
        const j = e.payload;
        setJobMap((m) => {
          const next = new Map(m);
          next.set(j.id, j);
          return next;
        });
        if (["done", "failed", "skipped", "cancelled"].includes(j.status)) reloadHistory().catch(() => {});
      }),
    );
    un.push(listen<boolean>("queue-paused", (e) => setPaused(e.payload)));
    un.push(listen<AppSettings>("settings-changed", (e) => setSettings(e.payload)));
    un.push(listen<string>("toast", (e) => toast("info", e.payload)));
    return () => {
      un.forEach((p) => p.then((f) => f()));
    };
  }, [reloadHistory, toast]);

  // Theme
  useEffect(() => {
    const apply = () => {
      const pref = settings?.theme ?? "system";
      const dark = pref === "dark" || (pref === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
      document.documentElement.dataset.theme = dark ? "dark" : "light";
    };
    apply();
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    mq.addEventListener("change", apply);
    return () => mq.removeEventListener("change", apply);
  }, [settings?.theme]);

  const addPaths = useCallback(async (paths: string[]) => {
    if (!paths.length) return [];
    const infos = await api.inspectFiles(paths);
    setFiles((cur) => {
      const seen = new Set(cur.map((f) => f.path.toLowerCase()));
      return [...cur, ...infos.filter((f) => !seen.has(f.path.toLowerCase()))];
    });
    return infos;
  }, []);

  const run = useCallback(
    async (paths: string[], a: ActionInfo | Action, options: Options = {}) => {
      const action = "action" in a ? a.action : a;
      try {
        const ids = await api.submitJob({ inputs: paths, action, options });
        toast("info", `${ids.length} job${ids.length === 1 ? "" : "s"} queued`, { label: "View", run: () => setView("jobs") });
        return true;
      } catch (e) {
        toast("err", errText(e));
        return false;
      }
    },
    [toast],
  );

  const jobs = useMemo(() => [...jobMap.values()].sort((a, b) => b.created.localeCompare(a.created)), [jobMap]);

  // Notify on completion of jobs
  const notified = useRef(new Set<string>());
  useEffect(() => {
    for (const j of jobs) {
      if (notified.current.has(j.id)) continue;
      if (j.status === "done") {
        notified.current.add(j.id);
        const out = j.outputs[0];
        toast("ok", `${j.label}: ${out ? out.split(/[\\/]/).pop() : "done"}${j.note ? ` · ${j.note}` : ""}`, out ? { label: "Show", run: () => api.revealPath(out).catch(() => {}) } : undefined);
      } else if (j.status === "failed") {
        notified.current.add(j.id);
        toast("err", `${j.label} failed: ${j.error ?? "unknown error"}`);
      } else if (j.status === "skipped") {
        notified.current.add(j.id);
        toast("info", j.note ?? "Skipped");
      } else if (j.status === "cancelled") {
        notified.current.add(j.id);
      }
    }
  }, [jobs, toast]);

  const saveSettings = useCallback(
    async (s: AppSettings) => {
      try {
        setSettings(await api.saveSettings(s));
      } catch (e) {
        toast("err", errText(e));
        setSettings(await api.getSettings());
      }
    },
    [toast],
  );

  const value: Store = {
    view,
    go: setView,
    settings,
    saveSettings,
    engines,
    refreshEngines,
    files,
    addPaths,
    removeFile: (p) => setFiles((f) => f.filter((x) => x.path !== p)),
    clearFiles: () => setFiles([]),
    preselect,
    setPreselect,
    jobs,
    paused,
    history,
    reloadHistory,
    run,
    toasts,
    toast,
    dismiss,
  };
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}
