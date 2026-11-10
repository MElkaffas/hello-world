import { useMemo, useState } from "react";
import { ask } from "@tauri-apps/plugin-dialog";
import { api } from "../api";
import { Icon } from "../components/Icon";
import { errText, useStore } from "../store";
import { fileName, formatDuration } from "../lib/util";

const pill: Record<string, string> = { done: "ok", failed: "err", skipped: "warn", interrupted: "warn", cancelled: "", running: "accent" };

export function HistoryView() {
  const { history, reloadHistory, toast, go } = useStore();
  const [q, setQ] = useState("");
  const [status, setStatus] = useState("all");
  const rows = useMemo(
    () =>
      history.filter(
        (h) =>
          (status === "all" || h.status === status) &&
          (!q || h.label.toLowerCase().includes(q.toLowerCase()) || h.inputs.some((p) => p.toLowerCase().includes(q.toLowerCase())) || h.outputs.some((p) => p.toLowerCase().includes(q.toLowerCase()))),
      ),
    [history, q, status],
  );

  const clear = async () => {
    if (await ask("Clear the whole conversion history? Converted files are not deleted.", { title: "Clear history", kind: "warning" })) {
      await api.clearHistory();
      await reloadHistory();
    }
  };

  return (
    <div className="page">
      <div className="page-head">
        <div>
          <h1>History</h1>
          <p>{history.length} conversion{history.length === 1 ? "" : "s"} recorded on this PC</p>
        </div>
        <div className="spacer" />
        <input className="input" placeholder="Search files or actions" value={q} onChange={(e) => setQ(e.target.value)} style={{ width: 220 }} />
        <select className="select" value={status} onChange={(e) => setStatus(e.target.value)}>
          {["all", "done", "failed", "skipped", "cancelled", "interrupted"].map((s) => <option key={s} value={s}>{s === "all" ? "All statuses" : s}</option>)}
        </select>
        <button className="btn danger" disabled={!history.length} onClick={clear}><Icon name="trash" size={15} /> Clear</button>
      </div>
      <div className="card">
        {rows.length === 0 ? (
          <div className="empty"><div className="e-icon"><Icon name="history" /></div>{history.length ? "No entries match." : "No conversions yet."}</div>
        ) : (
          <table className="tbl">
            <thead>
              <tr><th>File</th><th>Action</th><th>Result</th><th>When</th><th style={{ width: 150 }} /></tr>
            </thead>
            <tbody>
              {rows.map((h) => (
                <tr key={h.id}>
                  <td style={{ maxWidth: 260 }}>
                    <div className="ellipsis" title={h.inputs.join("\n")}>{fileName(h.inputs[0] ?? "")}{h.inputs.length > 1 ? ` +${h.inputs.length - 1}` : ""}</div>
                    {h.outputs[0] && <div className="small faint ellipsis" title={h.outputs[0]}>→ {fileName(h.outputs[0])}</div>}
                  </td>
                  <td className="small">{h.label}</td>
                  <td>
                    <span className={`pill ${pill[h.status] ?? ""}`}>{h.status}</span>
                    {(h.error || h.note) && <div className="small faint selectable" style={{ maxWidth: 280, marginTop: 3 }}>{h.error ?? h.note}</div>}
                  </td>
                  <td className="small faint">{new Date(h.time).toLocaleString()}{h.elapsed_ms ? <div>{formatDuration(h.elapsed_ms / 1000)}</div> : null}</td>
                  <td>
                    <div className="row" style={{ justifyContent: "flex-end", gap: 2 }}>
                      {h.outputs[0] && <button className="icon-btn" title="Show in Explorer" onClick={() => api.revealPath(h.outputs[0]).catch((e) => toast("err", errText(e)))}><Icon name="reveal" size={16} /></button>}
                      {h.spec && (
                        <button className="icon-btn" title="Convert again" onClick={async () => {
                          try {
                            await api.reconvert(h.id);
                            toast("info", "Queued again", { label: "View", run: () => go("jobs") });
                          } catch (e) {
                            toast("err", errText(e));
                          }
                        }}><Icon name="repeat" size={16} /></button>
                      )}
                      <button className="icon-btn" title="Remove from history" onClick={async () => { await api.removeHistory(h.id); await reloadHistory(); }}><Icon name="x" size={15} /></button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
