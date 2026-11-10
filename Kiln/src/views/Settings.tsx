import { useEffect, useState } from "react";
import { ask, open } from "@tauri-apps/plugin-dialog";
import { api, type AppSettings } from "../api";
import { Icon } from "../components/Icon";
import { errText, useStore } from "../store";
import { previewName } from "../lib/util";

function Toggle({ title, desc, checked, onChange }: { title: string; desc?: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="toggle">
      <div>
        <div className="t-title">{title}</div>
        {desc && <div className="t-desc">{desc}</div>}
      </div>
      <span className="switch">
        <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
        <span />
      </span>
    </label>
  );
}

export function SettingsView() {
  const { settings, saveSettings, engines, refreshEngines, toast } = useStore();
  const [ctxMenu, setCtxMenu] = useState<boolean | null>(null);
  const [checking, setChecking] = useState(false);
  const [draftShortcuts, setDraftShortcuts] = useState({ dz: "", sel: "" });

  useEffect(() => {
    api.contextMenuStatus().then(setCtxMenu).catch(() => setCtxMenu(null));
  }, []);
  useEffect(() => {
    if (settings) setDraftShortcuts({ dz: settings.shortcut_dropzone, sel: settings.shortcut_selection });
  }, [settings]);

  if (!settings) return <div className="page">Loading…</div>;
  const s = settings;
  const save = (patch: Partial<AppSettings>) => saveSettings({ ...s, ...patch });
  const setOut = (patch: Partial<AppSettings["output"]>) => save({ output: { ...s.output, ...patch } });

  return (
    <div className="page">
      <div className="page-head"><div><h1>Settings</h1><p>Changes are saved immediately.</p></div></div>

      <div className="grid c2" style={{ alignItems: "start" }}>
        <div className="card card-pad">
          <h3>Output</h3>
          <div className="field" style={{ marginBottom: 14 }}>
            <label>Save converted files</label>
            <div className="seg">
              <button className={!s.output.directory ? "on" : ""} onClick={() => setOut({ directory: null })}>Beside the original</button>
              <button className={s.output.directory ? "on" : ""} onClick={async () => {
                const d = await open({ directory: true });
                if (typeof d === "string") setOut({ directory: d });
              }}>In a folder…</button>
            </div>
            {s.output.directory && <div className="small mono faint ellipsis" title={s.output.directory}>{s.output.directory}</div>}
          </div>
          <div className="field" style={{ marginBottom: 14 }}>
            <label>File name</label>
            <input className="input" value={s.output.template} onChange={(e) => setOut({ template: e.target.value })} />
            <div className="small faint">Tokens: {"{name} {ext} {date} {time}"} · Example: {previewName(s.output.template, "Holiday.heic", "jpg")}</div>
          </div>
          <div className="field">
            <label>If a file with that name exists</label>
            <select className="select" value={s.output.conflict} onChange={async (e) => {
              const v = e.target.value as AppSettings["output"]["conflict"];
              if (v === "overwrite" && !(await ask("Replace existing files with the same name? Original source files are always protected, but other files in the output folder can be overwritten.", { title: "Allow overwriting", kind: "warning" }))) return;
              setOut({ conflict: v });
            }}>
              <option value="rename">Keep both — add a number (recommended)</option>
              <option value="skip">Skip the file</option>
              <option value="overwrite">Replace the existing file</option>
            </select>
          </div>
        </div>

        <div className="card card-pad">
          <h3>Behaviour</h3>
          <div className="field" style={{ marginBottom: 6 }}>
            <label>Theme</label>
            <div className="seg">
              {(["system", "light", "dark"] as const).map((t) => <button key={t} className={s.theme === t ? "on" : ""} onClick={() => save({ theme: t })}>{t[0].toUpperCase() + t.slice(1)}</button>)}
            </div>
          </div>
          <div className="field" style={{ margin: "12px 0 6px" }}>
            <label>Conversions at the same time: {s.max_parallel}</label>
            <input type="range" min={1} max={8} value={s.max_parallel} onChange={(e) => save({ max_parallel: Number(e.target.value) })} />
          </div>
          <Toggle title="Show the action wheel when files are dropped" desc="Off: dropped files go straight to the workspace" checked={s.radial_on_drop} onChange={(v) => save({ radial_on_drop: v })} />
          <Toggle title="Keep running in the tray when closed" desc="The drop zone and shortcuts keep working" checked={s.close_to_tray} onChange={(v) => save({ close_to_tray: v })} />
          <Toggle title="Start with Windows" desc="Starts minimized to the tray" checked={s.launch_at_startup} onChange={(v) => save({ launch_at_startup: v })} />
          {ctxMenu !== null && (
            <Toggle title="Explorer context menu" desc="“Convert with Mostafa-converting app” on files and “Compress with Mostafa-converting app” on folders. On Windows 11 it is under “Show more options”." checked={ctxMenu} onChange={async (v) => {
              try {
                await api.setContextMenu(v);
                setCtxMenu(v);
              } catch (e) {
                toast("err", errText(e));
              }
            }} />
          )}
        </div>
      </div>

      <div className="card card-pad" style={{ marginTop: 16 }}>
        <h3>Global shortcuts</h3>
        <div className="grid c2">
          <div className="field">
            <label>Show / hide the floating drop zone</label>
            <div className="row">
              <input className="input grow" value={draftShortcuts.dz} onChange={(e) => setDraftShortcuts((d) => ({ ...d, dz: e.target.value }))} placeholder="e.g. Ctrl+Alt+K" />
            </div>
          </div>
          <div className="field">
            <label>Open the wheel for files selected in Explorer</label>
            <input className="input" value={draftShortcuts.sel} onChange={(e) => setDraftShortcuts((d) => ({ ...d, sel: e.target.value }))} placeholder="e.g. Ctrl+Alt+Shift+K" />
          </div>
        </div>
        <div className="row" style={{ marginTop: 12 }}>
          <button className="btn" disabled={draftShortcuts.dz === s.shortcut_dropzone && draftShortcuts.sel === s.shortcut_selection} onClick={() => save({ shortcut_dropzone: draftShortcuts.dz, shortcut_selection: draftShortcuts.sel })}>Apply shortcuts</button>
          <span className="small faint">Leave empty to disable. Use names like Ctrl, Alt, Shift and a key.</span>
        </div>
      </div>

      <div className="card" style={{ marginTop: 16 }}>
        <div className="row card-pad" style={{ paddingBottom: 6 }}>
          <h3 className="grow" style={{ margin: 0 }}>Conversion engines</h3>
          <button className="btn sm" disabled={checking} onClick={async () => { setChecking(true); await refreshEngines(); setChecking(false); }}>
            <Icon name="refresh" size={14} /> {checking ? "Checking…" : "Re-check"}
          </button>
        </div>
        <div className="small faint" style={{ padding: "0 20px 10px" }}>
          The app only runs these programs and verifies each one by its version output. They ship inside the app (“bundled”), so a fresh PC needs no extra installs; Microsoft Office is the exception. Formats whose engine is missing are hidden — nothing is downloaded silently.
        </div>
        <table className="tbl">
          <thead><tr><th>Engine</th><th>Status</th><th>Version / path</th><th /></tr></thead>
          <tbody>
            {engines.map((e) => (
              <tr key={e.id}>
                <td>{e.label}</td>
                <td><span className={`pill ${e.available ? "ok" : "warn"}`}>{e.available ? "ready" : "missing"}</span></td>
                <td className="small" style={{ maxWidth: 420 }}>
                  <div className="ellipsis">{e.version ?? e.problem}</div>
                  {e.path && <div className="faint mono ellipsis" title={e.path}>{e.path}</div>}
                </td>
                <td style={{ textAlign: "right" }}>
                  {!e.available && e.winget_id && (
                    <button className="btn sm" onClick={async () => {
                      if (await ask(`Install ${e.label} with Windows Package Manager?\n\nA console window opens and runs:\nwinget install --id ${e.winget_id} -e\n\nwinget verifies the installer and shows its license. Click Re-check afterwards.`, { title: "Install engine" })) {
                        api.installEngine(e.id).catch((err) => toast("err", errText(err)));
                      }
                    }}>Install…</button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
