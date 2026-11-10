import { useEffect, useMemo, useState } from "react";
import { api, type ActionInfo, type Options } from "../api";
import { CategoryIcon, Icon } from "../components/Icon";
import { OptionsPanel } from "../components/OptionsPanel";
import { errText, useStore } from "../store";
import { actionKey, categoryLabels, extOf, formatBytes, groupConversions, previewName, sameAction, validateOptions } from "../lib/util";

export function Workspace({ pickFiles, pickFolder }: { pickFiles: () => Promise<string[]>; pickFolder: () => Promise<void> }) {
  const { files, removeFile, clearFiles, run, settings, preselect, setPreselect, go } = useStore();
  const [actions, setActions] = useState<ActionInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<"convert" | "tool">("convert");
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<ActionInfo | null>(null);
  const [options, setOptions] = useState<Options>({});
  const [busy, setBusy] = useState(false);

  const usable = useMemo(() => files.filter((f) => !f.error), [files]);
  const usableKey = usable.map((f) => f.path).join("|");

  useEffect(() => {
    let alive = true;
    if (!usable.length) {
      setActions([]);
      setSelected(null);
      return;
    }
    setLoading(true);
    setError(null);
    api
      .getActions(usable.map((f) => f.path))
      .then((a) => {
        if (!alive) return;
        setActions(a);
        // keep the selection if it is still valid, otherwise honour a preselected tool
        setSelected((cur) => {
          if (preselect) {
            const p = a.find((x) => sameAction(x.action, preselect));
            if (p) {
              setTab(p.group);
              setPreselect(null);
              return p;
            }
          }
          return cur ? a.find((x) => actionKey(x.action) === actionKey(cur.action) && x.group === cur.group) ?? null : null;
        });
      })
      .catch((e) => alive && setError(errText(e)))
      .finally(() => alive && setLoading(false));
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [usableKey]);

  const groups = useMemo(() => groupConversions(actions, query), [actions, query]);
  const tools = useMemo(() => actions.filter((a) => a.group === "tool" && (!query || a.label.toLowerCase().includes(query.toLowerCase()))), [actions, query]);
  const problem = selected ? validateOptions(selected, options) : null;
  const totalSize = usable.reduce((s, f) => s + f.size, 0);
  const mixed = new Set(usable.map((f) => f.category ?? "folder")).size > 1;

  const choose = (a: ActionInfo) => {
    setSelected(a);
    setOptions({});
  };

  const convert = async () => {
    if (!selected || problem) return;
    setBusy(true);
    const ok = await run(usable.map((f) => f.path), selected, options);
    setBusy(false);
    if (ok) {
      clearFiles();
      setSelected(null);
      go("jobs");
    }
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === "Enter") convert();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const outLabel = settings?.output.directory ? settings.output.directory : "Beside each original file";
  const exampleExt = extOf(selected?.action.to);

  return (
    <div className="page">
      <div className="page-head">
        <div>
          <h1>Workspace</h1>
          <p>{files.length ? `${files.length} item${files.length > 1 ? "s" : ""} · ${formatBytes(totalSize)}` : "Add files to see what you can do with them."}</p>
        </div>
        <div className="spacer" />
        <button className="btn" onClick={() => pickFiles()}><Icon name="plus" size={16} /> Add files</button>
        <button className="btn" onClick={() => pickFolder()}><Icon name="folder" size={16} /> Add folder</button>
        {files.length > 0 && <button className="btn ghost" onClick={() => { clearFiles(); setSelected(null); }}>Clear</button>}
      </div>

      {files.length === 0 ? (
        <div className="card empty">
          <div className="e-icon"><Icon name="upload" /></div>
          Drop files or folders here, or use <b>Add files</b>.
        </div>
      ) : (
        <div className="ws">
          <div className="card file-list">
            {files.map((f) => (
              <div key={f.path} className="file-row">
                <CategoryIcon category={f.is_dir ? "folder" : f.category} size={34} />
                <div className="grow" style={{ minWidth: 0 }}>
                  <div className="f-name ellipsis" title={f.path}>{f.name}</div>
                  <div className="f-meta">
                    {f.is_dir ? "Folder" : f.format_label ?? "Unknown"} · {formatBytes(f.size)}
                    {f.extension_mismatch && <span className="pill warn" title="The file content does not match its extension. Mostafa-converting app uses the real format.">really {f.format_label}</span>}
                  </div>
                  {f.error && <div className="small" style={{ color: "var(--err)" }}>{f.error}</div>}
                </div>
                <button className="icon-btn" onClick={() => removeFile(f.path)} aria-label={`Remove ${f.name}`}><Icon name="x" size={15} /></button>
              </div>
            ))}
          </div>

          <div>
            <div className="row" style={{ marginBottom: 14 }}>
              <div className="seg">
                <button className={tab === "convert" ? "on" : ""} onClick={() => setTab("convert")}>Convert to</button>
                <button className={tab === "tool" ? "on" : ""} onClick={() => setTab("tool")}>Tools</button>
              </div>
              <div className="grow" />
              <div className="row" style={{ position: "relative" }}>
                <span style={{ position: "absolute", left: 10, color: "var(--text-3)", display: "flex" }}><Icon name="search" size={15} /></span>
                <input className="input" style={{ paddingLeft: 32, width: 220 }} placeholder="Search formats and tools" value={query} onChange={(e) => setQuery(e.target.value)} />
              </div>
            </div>

            {error && <div className="alert err">{error}</div>}
            {mixed && !loading && <div className="alert info" style={{ marginBottom: 12 }}>Mixed selection: only options that work for every file are shown.</div>}
            {loading && <div className="small faint">Inspecting files…</div>}
            {!loading && usable.length === 0 && <div className="alert warn">None of the added files can be processed.</div>}
            {!loading && usable.length > 0 && actions.length === 0 && !error && (
              <div className="alert warn">No conversions are available for this selection with the installed engines. Check Settings → Engines.</div>
            )}

            {tab === "convert" &&
              groups.map((g) => (
                <div key={g.category} style={{ marginBottom: 16 }}>
                  <h3>{g.category === "other" ? "Other" : categoryLabels[g.category]}</h3>
                  <div className="fmt-grid">
                    {g.items.map((a) => (
                      <button key={actionKey(a.action)} className={`fmt ${selected && sameAction(selected.action, a.action) && selected.group === a.group ? "on" : ""}`} onClick={() => choose(a)}>
                        {a.recommended && <span className="star" title="Recommended"><Icon name="star" size={13} fill /></span>}
                        <div className="fmt-name">{a.label.replace(/ \(.*\)$/, "")}</div>
                        <div className="fmt-sub">{/\((.*)\)/.exec(a.label)?.[1] ?? (a.recommended ? "Recommended" : `.${extOf(a.action.to)}`)}</div>
                      </button>
                    ))}
                  </div>
                </div>
              ))}
            {tab === "convert" && !loading && groups.length === 0 && actions.length > 0 && <div className="small faint">No formats match “{query}”.</div>}

            {tab === "tool" && (
              <div className="tool-list">
                {tools.map((a) => (
                  <button key={`${actionKey(a.action)}-${a.label}`} className={`tile ${selected && sameAction(selected.action, a.action) && selected.group === "tool" ? "fmt on" : ""}`} onClick={() => choose(a)}>
                    <CategoryIcon category={a.output_category ?? "folder"} size={30} />
                    <div className="grow">
                      <div className="t-main">{a.label}</div>
                      <div className="t-sub">{a.combines_inputs ? "Combines all files" : a.produces_folder ? "Creates a folder" : "Saves a copy"}</div>
                    </div>
                  </button>
                ))}
                {!loading && tools.length === 0 && <div className="small faint">No tools for this selection.</div>}
              </div>
            )}

            {selected && (
              <div className="settings-box">
                <div className="row">
                  <b>{selected.label}</b>
                  <span className="small faint">via {selected.engines.join(" + ")}</span>
                </div>
                <OptionsPanel info={selected} options={options} set={(p) => setOptions((o) => ({ ...o, ...p }))} />
              </div>
            )}

            <div className="convert-bar">
              <div className="grow small">
                <div className="ellipsis"><span className="faint">Save to:</span> {outLabel}</div>
                {selected && usable[0] && selected.group === "convert" && !selected.produces_folder && exampleExt && (
                  <div className="faint ellipsis">e.g. {previewName(settings?.output.template ?? "{name}", usable[0].name, exampleExt)} · existing files are never overwritten</div>
                )}
                {problem && <div style={{ color: "var(--err)" }}>{problem}</div>}
              </div>
              <button className="btn primary lg" disabled={!selected || !!problem || busy} onClick={convert} title="Ctrl+Enter">
                {busy ? "Starting…" : selected ? `${selected.group === "convert" ? "Convert" : "Run"} ${usable.length > 1 && !selected.combines_inputs ? `${usable.length} files` : ""}` : "Choose an option"}
                <Icon name="arrow" size={16} />
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
