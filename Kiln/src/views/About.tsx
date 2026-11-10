import { useEffect, useState } from "react";
import { api, type AppInfo } from "../api";
import { Icon } from "../components/Icon";
import { useStore } from "../store";

export function AboutView() {
  const { engines } = useStore();
  const [info, setInfo] = useState<AppInfo | null>(null);
  useEffect(() => {
    api.appInfo().then(setInfo).catch(() => {});
  }, []);
  return (
    <div className="page">
      <div className="row" style={{ gap: 18, marginBottom: 26 }}>
        <div className="brand-mark" style={{ width: 64, height: 64, borderRadius: 18 }}><Icon name="drop" size={32} stroke={2.2} /></div>
        <div>
          <h1 style={{ fontFamily: "var(--font-display)", fontSize: 28, margin: 0 }}>Mostafa-converting app</h1>
          <div className="muted">Version {info?.version ?? "…"} · Windows 10/11 x64 · Tauri {info?.tauri ?? ""}</div>
        </div>
      </div>
      <div className="grid c2" style={{ alignItems: "start" }}>
        <div className="card card-pad">
          <h3>Privacy</h3>
          <ul style={{ margin: 0, paddingLeft: 18, lineHeight: 1.7 }}>
            <li>Every conversion runs on this PC. Files are never uploaded.</li>
            <li>No account, no telemetry, no analytics.</li>
            <li>The app makes no network requests. All conversion engines are included, so it works fully offline.</li>
            <li>Original files are never modified. Outputs are written to a private temp folder, verified, then moved into place.</li>
          </ul>
        </div>
        <div className="card card-pad">
          <h3>Data on this PC</h3>
          <div className="small faint">Settings & history</div>
          <div className="mono selectable" style={{ marginBottom: 10, wordBreak: "break-all" }}>{info?.data_dir}</div>
          <div className="small faint">Temporary files (cleaned automatically)</div>
          <div className="mono selectable" style={{ wordBreak: "break-all" }}>{info?.temp_dir}</div>
        </div>
      </div>
      <div className="card card-pad" style={{ marginTop: 16 }}>
        <h3>Third-party engines</h3>
        <p className="small muted" style={{ marginTop: 0 }}>
          The app ships with these open-source tools (in its “engines” folder) and uses Microsoft Office when it is installed. Each remains under its own license: FFmpeg (LGPL/GPL), ImageMagick (ImageMagick License), Pandoc (GPL), Poppler (GPL), qpdf (Apache 2.0), 7-Zip (LGPL + unRAR restriction), Tesseract (Apache 2.0).
        </p>
        <div className="row wrap" style={{ gap: 6 }}>
          {engines.map((e) => <span key={e.id} className={`pill ${e.available ? "ok" : ""}`}>{e.label}</span>)}
        </div>
      </div>
      <div className="small faint" style={{ marginTop: 18 }}>© 2026 Mostafa · MIT License. An independent project, not affiliated with any other file-conversion app.</div>
    </div>
  );
}
