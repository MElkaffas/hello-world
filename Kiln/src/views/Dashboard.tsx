import { open } from "@tauri-apps/plugin-dialog";
import { api, type Action, type Category } from "../api";
import { CategoryIcon, Icon } from "../components/Icon";
import { useStore } from "../store";
import { categoryLabels, categoryOrder, fileName } from "../lib/util";

const quick: { label: string; from: string; exts: string[]; action: Action; engine: string }[] = [
  { label: "HEIC → JPG", from: "iPhone photos", exts: ["heic", "heif"], action: { kind: "convert", to: "Jpg" }, engine: "magick" },
  { label: "PNG → WebP", from: "Smaller web images", exts: ["png"], action: { kind: "convert", to: "Webp" }, engine: "magick" },
  { label: "Video → GIF", from: "Shareable animation", exts: ["mp4", "mov", "mkv", "webm", "avi"], action: { kind: "convert", to: "Gif" }, engine: "ffmpeg" },
  { label: "PDF → JPG", from: "One image per page", exts: ["pdf"], action: { kind: "convert", to: "Jpg" }, engine: "pdftoppm" },
  { label: "MP4 → MP3", from: "Extract the audio", exts: ["mp4", "mov", "mkv", "avi", "webm"], action: { kind: "convert", to: "Mp3" }, engine: "ffmpeg" },
  { label: "WAV → MP3", from: "Compress audio", exts: ["wav", "aiff", "aif", "flac"], action: { kind: "convert", to: "Mp3" }, engine: "ffmpeg" },
  { label: "Word → PDF", from: "DOCX, DOC, ODT, RTF", exts: ["docx", "doc", "odt", "rtf"], action: { kind: "convert", to: "Pdf" }, engine: "word" },
  { label: "Excel → PDF", from: "XLSX, XLS, ODS", exts: ["xlsx", "xls", "ods"], action: { kind: "convert", to: "Pdf" }, engine: "excel" },
];

const catEngines: Record<Category, string[]> = {
  image: ["magick"],
  pdf: ["pdftoppm", "qpdf"],
  audio: ["ffmpeg"],
  video: ["ffmpeg"],
  document: ["pandoc", "word"],
  spreadsheet: ["excel"],
  presentation: ["powerpoint"],
  archive: ["sevenzip"],
};
const catFormats: Record<Category, string> = {
  image: "JPG PNG WebP HEIC AVIF TIFF BMP GIF SVG ICO",
  pdf: "Images, text, merge, split, OCR, protect",
  audio: "MP3 WAV AAC M4A FLAC OGG Opus AIFF WMA",
  video: "MP4 MOV MKV AVI WebM WMV MPEG GIF",
  document: "DOCX DOC ODT RTF TXT HTML MD EPUB",
  spreadsheet: "XLSX XLS ODS CSV",
  presentation: "PPTX PPT ODP → PDF, images",
  archive: "ZIP 7Z TAR GZ · extract RAR",
};

export function Dashboard({ pickFiles, pickFolder }: { pickFiles: () => Promise<string[]>; pickFolder: () => Promise<void> }) {
  const { history, jobs, engines, run, go, paused } = useStore();
  const has = (id: string) => engines.find((e) => e.id === id)?.available ?? false;
  const recent = history.filter((h) => h.status === "done").slice(0, 6);
  const running = jobs.filter((j) => j.status === "running" || j.status === "queued");

  const quickRun = async (q: (typeof quick)[number]) => {
    const sel = await open({ multiple: true, filters: [{ name: q.label, extensions: q.exts }] });
    const paths = Array.isArray(sel) ? sel : sel ? [sel] : [];
    if (paths.length) await run(paths, q.action);
  };

  return (
    <div className="page">
      <div className="drop">
        <div className="drop-icon"><Icon name="upload" size={30} stroke={2} /></div>
        <h2>Drop files anywhere in this window</h2>
        <div className="steps"><b>Drag</b>→<b>Choose</b>→<b>Convert</b>→<b>Done</b></div>
        <div className="row" style={{ justifyContent: "center" }}>
          <button className="btn primary lg" onClick={() => pickFiles()}><Icon name="plus" size={17} /> Add files</button>
          <button className="btn lg" onClick={() => pickFolder()}><Icon name="folder" size={17} /> Add folder</button>
        </div>
        <div className="small faint" style={{ marginTop: 16 }}>
          Ctrl+O add files · Ctrl+Shift+O add folder · right-click files in Explorer → <b>Convert with Mostafa-converting app</b>
        </div>
      </div>

      <div className="row" style={{ marginTop: 16, gap: 8 }}>
        <span className="pill ok"><Icon name="shield" size={12} /> Everything runs on this PC</span>
        {running.length > 0 ? (
          <button className="pill accent" style={{ border: 0, cursor: "pointer" }} onClick={() => go("jobs")}>{running.length} job{running.length > 1 ? "s" : ""} in progress</button>
        ) : (
          <span className="pill">No active jobs</span>
        )}
        {paused && <span className="pill warn">Queue paused</span>}
      </div>

      <h2>Quick conversions</h2>
      <div className="grid c4">
        {quick.map((q) => {
          const ok = has(q.engine);
          return (
            <button key={q.label} className={`tile ${ok ? "" : "disabled"}`} onClick={() => (ok ? quickRun(q) : go("settings"))} title={ok ? `Choose ${q.exts.join(", ")} files` : "The required engine is not installed — open Settings"}>
              <div className="t-icon" style={{ background: "var(--accent-soft)", color: "var(--accent)" }}><Icon name="bolt" size={17} /></div>
              <div className="grow">
                <div className="t-main">{q.label}</div>
                <div className="t-sub">{ok ? q.from : "Engine not installed"}</div>
              </div>
            </button>
          );
        })}
      </div>

      <div className="grid c2" style={{ marginTop: 8, alignItems: "start" }}>
        <div>
          <h2>Recent conversions</h2>
          <div className="card">
            {recent.length === 0 ? (
              <div className="empty"><div className="e-icon"><Icon name="history" /></div>Converted files will show up here.</div>
            ) : (
              recent.map((h) => (
                <div key={h.id} className="file-row">
                  <Icon name="check" size={16} />
                  <div className="grow">
                    <div className="f-name ellipsis">{fileName(h.outputs[0] ?? h.inputs[0])}</div>
                    <div className="f-meta">{h.label} · {new Date(h.time).toLocaleString()}</div>
                  </div>
                  {h.outputs[0] && <button className="icon-btn" title="Show in Explorer" onClick={() => api.revealPath(h.outputs[0]).catch(() => {})}><Icon name="reveal" size={16} /></button>}
                </div>
              ))
            )}
          </div>
        </div>
        <div>
          <h2>What Mostafa-converting app handles</h2>
          <div className="grid c2">
            {categoryOrder.map((c) => {
              const ok = catEngines[c].some(has);
              return (
                <div key={c} className={`tile ${ok ? "" : "disabled"}`} style={{ cursor: "default" }} title={ok ? "" : "Install the engine in Settings → Engines"}>
                  <CategoryIcon category={c} />
                  <div className="grow">
                    <div className="t-main row">{categoryLabels[c]} {!ok && <span className="pill warn">setup</span>}</div>
                    <div className="t-sub ellipsis">{catFormats[c]}</div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
