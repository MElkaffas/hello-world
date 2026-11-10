import { useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { api, type ActionInfo, type AppSettings, type JobSnapshot } from "../api";
import { Icon } from "../components/Icon";
import { RadialMenu } from "../components/RadialMenu";

/** Floating, always-on-top drop target. Drop files (or press the Explorer-selection shortcut)
 *  and an action wheel opens right here; progress is shown on the puck. */
export function DropZone() {
  const [over, setOver] = useState(false);
  const [paths, setPaths] = useState<string[] | null>(null);
  const [actions, setActions] = useState<ActionInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [batch, setBatch] = useState<string[]>([]);
  const [jobs, setJobs] = useState<Map<string, JobSnapshot>>(new Map());
  const [msg, setMsg] = useState<string | null>(null);
  const batchRef = useRef<string[]>([]);

  useEffect(() => {
    api.getSettings().then((s: AppSettings) => {
      const dark = s.theme === "dark" || (s.theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
      document.documentElement.dataset.theme = dark ? "dark" : "light";
    }).catch(() => {});
  }, []);

  const openWheel = async (p: string[]) => {
    setPaths(p);
    setActions([]);
    setLoading(true);
    setMsg(null);
    try {
      setActions(await api.getActions(p));
    } catch (e) {
      setMsg(String(e));
      setPaths(null);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    const un1 = getCurrentWebview().onDragDropEvent((e) => {
      const t = e.payload.type;
      if (t === "enter" || t === "over") setOver(true);
      else if (t === "leave") setOver(false);
      else if (t === "drop") {
        setOver(false);
        if (e.payload.paths.length) openWheel(e.payload.paths);
      }
    });
    const un2 = listen<string[]>("wheel-files", (e) => openWheel(e.payload));
    const un3 = listen<JobSnapshot>("job", (e) => {
      if (batchRef.current.includes(e.payload.id)) setJobs((m) => new Map(m).set(e.payload.id, e.payload));
    });
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !paths) getCurrentWindow().hide();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      un1.then((f) => f());
      un2.then((f) => f());
      un3.then((f) => f());
      window.removeEventListener("keydown", onKey);
    };
  }, [paths]);

  const mine = batch.map((id) => jobs.get(id)).filter(Boolean) as JobSnapshot[];
  const running = mine.filter((j) => j.status === "queued" || j.status === "running");
  const failed = mine.filter((j) => j.status === "failed");
  const done = mine.filter((j) => j.status === "done");
  const progress = mine.length ? mine.reduce((s, j) => s + (j.status === "running" ? j.progress ?? 0 : j.status === "queued" ? 0 : 1), 0) / mine.length : 0;

  let title = "Drop files";
  let sub = "to convert";
  if (running.length) {
    title = `${Math.round(progress * 100)}%`;
    sub = `${running.length} working`;
  } else if (mine.length && failed.length) {
    title = "Failed";
    sub = failed[0].error?.slice(0, 40) ?? "";
  } else if (mine.length) {
    title = "Done";
    sub = "click to show";
  }

  const C = 2 * Math.PI * 70;
  return (
    <div className="dz">
      <div
        className={`dz-puck ${over ? "over" : ""}`}
        data-tauri-drag-region
        onDoubleClick={() => api.showMain()}
        onClick={() => {
          if (!running.length && done[0]?.outputs[0]) api.revealPath(done[0].outputs[0]).catch(() => {});
        }}
        title="Drag to move · double-click to open Mostafa-converting app · Esc to hide"
      >
        <div data-tauri-drag-region style={{ pointerEvents: "none" }}>
          {mine.length === 0 && <Icon name="upload" size={26} stroke={2} />}
          {mine.length > 0 && !running.length && <Icon name={failed.length ? "alert" : "check"} size={26} stroke={2.2} />}
          <div className="dz-title">{title}</div>
          <div className="dz-sub">{msg ?? sub}</div>
        </div>
      </div>
      {running.length > 0 && (
        <svg className="dz-ring" style={{ position: "fixed", left: "50%", top: "50%", transform: "translate(-50%,-50%) rotate(-90deg)", pointerEvents: "none" }} viewBox="0 0 150 150">
          <circle cx="75" cy="75" r="70" fill="none" stroke="rgba(255,255,255,.25)" strokeWidth="5" />
          <circle cx="75" cy="75" r="70" fill="none" stroke="white" strokeWidth="5" strokeDasharray={C} strokeDashoffset={C * (1 - progress)} strokeLinecap="round" style={{ transition: "stroke-dashoffset .3s" }} />
        </svg>
      )}
      <button className="dz-close" onClick={() => getCurrentWindow().hide()} aria-label="Hide drop zone"><Icon name="x" size={12} /></button>
      {paths && (
        <RadialMenu
          x={window.innerWidth / 2}
          y={window.innerHeight / 2}
          size={290}
          actions={actions}
          loading={loading}
          count={paths.length}
          onClose={() => setPaths(null)}
          onMore={() => {
            api.showMain(paths);
            setPaths(null);
          }}
          onPick={async (a) => {
            const p = paths;
            setPaths(null);
            try {
              const ids = await api.submitJob({ inputs: p, action: a.action, options: {} });
              batchRef.current = ids;
              setBatch(ids);
              setJobs(new Map());
            } catch (e) {
              setMsg(String(e).slice(0, 60));
            }
          }}
        />
      )}
    </div>
  );
}
