import { api, type JobSnapshot } from "../api";
import { Icon } from "../components/Icon";
import { errText, useStore } from "../store";
import { fileName, formatDuration } from "../lib/util";

const statusPill: Record<JobSnapshot["status"], string> = {
  queued: "",
  running: "accent",
  done: "ok",
  skipped: "warn",
  failed: "err",
  cancelled: "",
};

export function JobRow({ j }: { j: JobSnapshot }) {
  const { toast } = useStore();
  const pct = j.progress != null ? Math.round(j.progress * 100) : null;
  const first = j.inputs[0] ? fileName(j.inputs[0]) : "";
  const reveal = (p: string) => api.revealPath(p).catch((e) => toast("err", errText(e)));
  return (
    <div className="job">
      <div style={{ minWidth: 0 }}>
        <div className="row">
          <span className="f-name ellipsis" title={j.inputs.join("\n")}>{first}{j.inputs.length > 1 ? ` + ${j.inputs.length - 1} more` : ""}</span>
          <span className={`pill ${statusPill[j.status]}`}>{j.status}</span>
        </div>
        <div className="small faint">
          {j.label}
          {j.status === "running" && <> · {j.stage}{pct != null ? ` · ${pct}%` : ""}{j.eta_secs != null ? ` · about ${formatDuration(j.eta_secs)} left` : ""}</>}
          {j.status === "done" && <> · {formatDuration(j.elapsed_ms / 1000)}{j.note ? ` · ${j.note}` : ""}</>}
          {j.status === "skipped" && j.note && <> · {j.note}</>}
        </div>
        {j.error && <div className="small selectable" style={{ color: "var(--err)", marginTop: 2 }}>{j.error}</div>}
      </div>
      <div className="row">
        {j.status === "done" && j.outputs[0] && (
          <>
            <button className="btn sm" onClick={() => api.openPath(j.outputs[0]).catch((e) => toast("err", errText(e)))}><Icon name="open" size={14} /> Open</button>
            <button className="btn sm" onClick={() => reveal(j.outputs[0])}><Icon name="reveal" size={14} /> Show</button>
          </>
        )}
        {(j.status === "queued" || j.status === "running") && (
          <button className="btn sm danger" onClick={() => api.cancelJob(j.id)}><Icon name="x" size={14} /> Cancel</button>
        )}
      </div>
      {(j.status === "running" || j.status === "queued") && (
        <div className={`progress ${j.status === "running" && pct == null ? "indeterminate" : ""}`}>
          <div style={{ width: `${pct ?? 0}%` }} />
        </div>
      )}
    </div>
  );
}

export function JobsView() {
  const { jobs, paused } = useStore();
  const active = jobs.filter((j) => j.status === "running" || j.status === "queued");
  const finished = jobs.filter((j) => !(j.status === "running" || j.status === "queued"));
  return (
    <div className="page">
      <div className="page-head">
        <div>
          <h1>Activity</h1>
          <p>{active.length ? `${active.length} active` : "Nothing running"}{finished.length ? ` · ${finished.length} finished this session` : ""}</p>
        </div>
        <div className="spacer" />
        <button className="btn" onClick={() => api.setPaused(!paused)} title="Paused queues finish running jobs but start no new ones">
          <Icon name={paused ? "play" : "pause"} size={15} /> {paused ? "Resume queue" : "Pause queue"}
        </button>
        {active.length > 0 && <button className="btn danger" onClick={() => api.cancelAll()}>Cancel all</button>}
        {finished.length > 0 && <button className="btn ghost" onClick={async () => { await api.clearFinished(); window.location.reload(); }}>Clear finished</button>}
      </div>
      {paused && <div className="alert warn" style={{ marginBottom: 12 }}>The queue is paused. Running conversions continue; queued ones wait until you resume.</div>}
      <div className="card">
        {jobs.length === 0 ? (
          <div className="empty"><div className="e-icon"><Icon name="jobs" /></div>Conversions you start appear here with live progress.</div>
        ) : (
          [...active, ...finished].map((j) => <JobRow key={j.id} j={j} />)
        )}
      </div>
    </div>
  );
}
