import { useCallback, useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { api, type ActionInfo } from "./api";
import { Icon } from "./components/Icon";
import { RadialMenu } from "./components/RadialMenu";
import { errText, useStore, type View } from "./store";
import { Dashboard } from "./views/Dashboard";
import { Workspace } from "./views/Workspace";
import { JobsView } from "./views/Jobs";
import { HistoryView } from "./views/History";
import { ToolsView } from "./views/Tools";
import { SettingsView } from "./views/Settings";
import { AboutView } from "./views/About";

const nav: { id: View; label: string; icon: string; key: string }[] = [
  { id: "home", label: "Home", icon: "home", key: "1" },
  { id: "workspace", label: "Workspace", icon: "workspace", key: "2" },
  { id: "jobs", label: "Activity", icon: "jobs", key: "3" },
  { id: "history", label: "History", icon: "history", key: "4" },
  { id: "tools", label: "Tools", icon: "tools", key: "5" },
  { id: "settings", label: "Settings", icon: "settings", key: "6" },
  { id: "about", label: "About", icon: "info", key: "7" },
];

interface Wheel { x: number; y: number; paths: string[]; actions: ActionInfo[]; loading: boolean }

export function useFilePickers() {
  const { addPaths, go, toast } = useStore();
  const pickFiles = useCallback(
    async (filters?: { name: string; extensions: string[] }[]) => {
      const sel = await open({ multiple: true, directory: false, filters });
      const paths = Array.isArray(sel) ? sel : sel ? [sel] : [];
      if (paths.length) {
        await addPaths(paths);
        go("workspace");
      }
      return paths;
    },
    [addPaths, go],
  );
  const pickFolder = useCallback(async () => {
    const sel = await open({ directory: true, multiple: false });
    if (typeof sel !== "string") return;
    try {
      const files = await api.listFolder(sel);
      if (!files.length) return toast("info", "That folder has no files.");
      await addPaths(files);
      if (files.length >= 2000) toast("info", "Added the first 2000 files of the folder.");
      go("workspace");
    } catch (e) {
      toast("err", errText(e));
    }
  }, [addPaths, go, toast]);
  return { pickFiles, pickFolder };
}

export function App() {
  const store = useStore();
  const { view, go, jobs, settings, addPaths, toasts, dismiss, run, paused, engines } = store;
  const [dragging, setDragging] = useState(false);
  const [wheel, setWheel] = useState<Wheel | null>(null);
  const { pickFiles, pickFolder } = useFilePickers();

  const openWheel = useCallback(
    async (paths: string[], x: number, y: number) => {
      await addPaths(paths);
      if (!settings?.radial_on_drop) {
        go("workspace");
        return;
      }
      const cx = Math.min(Math.max(x, 190), window.innerWidth - 190);
      const cy = Math.min(Math.max(y, 190), window.innerHeight - 210);
      setWheel({ x: cx, y: cy, paths, actions: [], loading: true });
      try {
        const actions = await api.getActions(paths);
        setWheel((w) => (w && w.paths === paths ? { ...w, actions, loading: false } : w));
      } catch (e) {
        setWheel(null);
        store.toast("err", errText(e));
        go("workspace");
      }
    },
    [addPaths, settings?.radial_on_drop, go, store],
  );

  // Native drag & drop from Explorer and other apps
  useEffect(() => {
    const un = getCurrentWebview().onDragDropEvent((e) => {
      const p = e.payload;
      if (p.type === "enter" || p.type === "over") setDragging(true);
      else if (p.type === "leave") setDragging(false);
      else if (p.type === "drop") {
        setDragging(false);
        if (!p.paths.length) {
          store.toast("info", "Only files can be dropped here. Save web images to disk first.");
          return;
        }
        const r = window.devicePixelRatio || 1;
        openWheel(p.paths, p.position.x / r, p.position.y / r);
      }
    });
    return () => {
      un.then((f) => f());
    };
  }, [openWheel, store]);

  // Files from the Explorer context menu / second instance / tray drop zone
  useEffect(() => {
    let buffer: string[] = [];
    let timer: number | undefined;
    const flush = () => {
      const paths = buffer;
      buffer = [];
      openWheel(paths, window.innerWidth / 2, window.innerHeight / 2);
    };
    const un = listen<string[]>("open-files", (e) => {
      // Explorer starts one process per selected file; collect them for a moment.
      buffer.push(...e.payload);
      window.clearTimeout(timer);
      timer = window.setTimeout(flush, 450);
    });
    api.takePendingFiles().then((p) => {
      if (p.length) {
        buffer.push(...p);
        timer = window.setTimeout(flush, 450);
      }
    });
    return () => {
      un.then((f) => f());
    };
  }, [openWheel]);

  // Keyboard shortcuts
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (wheel) return;
      if (e.ctrlKey && !e.shiftKey && e.key.toLowerCase() === "o") {
        e.preventDefault();
        pickFiles();
      } else if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "o") {
        e.preventDefault();
        pickFolder();
      } else if (e.ctrlKey && /^[1-7]$/.test(e.key)) {
        e.preventDefault();
        go(nav[parseInt(e.key, 10) - 1].id);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [wheel, pickFiles, pickFolder, go]);

  const active = jobs.filter((j) => j.status === "queued" || j.status === "running").length;
  const missing = engines.filter((e) => !e.available && !["word", "excel", "powerpoint", "ffprobe", "pdfinfo", "pdftohtml", "pdftotext"].includes(e.id)).length;

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark"><Icon name="drop" size={17} stroke={2.2} /></div>
          <div>
            <div className="brand-name">Mostafa</div>
            <div className="small faint" style={{ marginTop: -2 }}>converting app</div>
          </div>
        </div>
        {nav.map((n) => (
          <button key={n.id} className={`nav-btn ${view === n.id ? "active" : ""}`} onClick={() => go(n.id)} title={`Ctrl+${n.key}`}>
            <Icon name={n.icon} size={17} />
            {n.label}
            {n.id === "jobs" && active > 0 && <span className="badge">{active}</span>}
            {n.id === "workspace" && store.files.length > 0 && <span className="badge" style={{ background: "var(--bg-sunken)", color: "var(--text-2)" }}>{store.files.length}</span>}
          </button>
        ))}
        <div className="sidebar-foot">
          <div className="row"><span className={`dot ${paused ? "warn" : "ok"}`} />{paused ? "Queue paused" : active ? `${active} active` : "Ready"}</div>
          <div className="row"><span className={`dot ${missing ? "warn" : "ok"}`} />{missing ? `${missing} engine${missing > 1 ? "s" : ""} missing` : "All engines ready"}</div>
          <div className="row"><Icon name="shield" size={13} /> Offline · nothing uploaded</div>
        </div>
      </aside>
      <main className="main">
        {view === "home" && <Dashboard pickFiles={pickFiles} pickFolder={pickFolder} />}
        {view === "workspace" && <Workspace pickFiles={pickFiles} pickFolder={pickFolder} />}
        {view === "jobs" && <JobsView />}
        {view === "history" && <HistoryView />}
        {view === "tools" && <ToolsView pickFiles={pickFiles} />}
        {view === "settings" && <SettingsView />}
        {view === "about" && <AboutView />}
      </main>

      {dragging && !wheel && (
        <div className="drag-overlay"><div>Drop to convert</div></div>
      )}
      {wheel && (
        <RadialMenu
          x={wheel.x}
          y={wheel.y}
          actions={wheel.actions}
          loading={wheel.loading}
          count={wheel.paths.length}
          onClose={() => setWheel(null)}
          onMore={() => {
            setWheel(null);
            go("workspace");
          }}
          onPick={async (a) => {
            const paths = wheel.paths;
            setWheel(null);
            if (await run(paths, a)) {
              paths.forEach((p) => store.removeFile(p));
            }
          }}
        />
      )}
      <div className="toasts" aria-live="polite">
        {toasts.map((t) => (
          <div className="toast" key={t.id}>
            <span style={{ color: t.kind === "err" ? "var(--err)" : t.kind === "ok" ? "var(--ok)" : "var(--accent)", marginTop: 1 }}>
              <Icon name={t.kind === "err" ? "alert" : t.kind === "ok" ? "check" : "info"} size={16} />
            </span>
            <div className="grow selectable" style={{ wordBreak: "break-word" }}>{t.text}</div>
            {t.action && (
              <button className="btn sm" onClick={() => { t.action!.run(); dismiss(t.id); }}>{t.action.label}</button>
            )}
            <button className="icon-btn" style={{ width: 22, height: 22 }} onClick={() => dismiss(t.id)} aria-label="Dismiss"><Icon name="x" size={13} /></button>
          </div>
        ))}
      </div>
    </div>
  );
}
