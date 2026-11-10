import { useEffect, useState } from "react";
import { api, type ActionInfo, type Options } from "../api";
import { parseTime } from "../lib/util";

type Setter = (patch: Partial<Options>) => void;

function Num({ value, onChange, placeholder, min, max, step }: { value: number | null | undefined; onChange: (v: number | null) => void; placeholder?: string; min?: number; max?: number; step?: number }) {
  return (
    <input
      className="input num"
      type="number"
      value={value ?? ""}
      placeholder={placeholder}
      min={min}
      max={max}
      step={step}
      onChange={(e) => onChange(e.target.value === "" ? null : Number(e.target.value))}
    />
  );
}

function Pair({ label, children, hint }: { label: string; children: React.ReactNode; hint?: string }) {
  return (
    <div className="pair">
      <span className="label">{label}</span>
      <div className="row wrap">
        {children}
        {hint && <span className="small faint">{hint}</span>}
      </div>
    </div>
  );
}

function TimeInput({ value, onChange, placeholder }: { value: number | null | undefined; onChange: (v: number | null) => void; placeholder: string }) {
  const [text, setText] = useState(value != null && !Number.isNaN(value) ? String(value) : "");
  return (
    <input
      className="input"
      style={{ width: 110 }}
      value={text}
      placeholder={placeholder}
      onChange={(e) => {
        setText(e.target.value);
        onChange(parseTime(e.target.value));
      }}
    />
  );
}

export function OptionsPanel({ info, options, set }: { info: ActionInfo; options: Options; set: Setter }) {
  const [langs, setLangs] = useState<string[]>([]);
  const p = info.params;
  const to = info.action.to;

  useEffect(() => {
    if (p.includes("ocr_language")) api.ocrLanguages().then(setLangs).catch(() => setLangs([]));
  }, [p]);

  if (p.length === 0) {
    return <div className="small faint">No settings needed — Mostafa-converting app picks sensible defaults for {info.label}.</div>;
  }

  return (
    <div style={{ display: "grid", gap: 12 }}>
      {p.includes("image_quality") && (
        <Pair label="Quality" hint={`${options.quality ?? (info.action.kind === "image_compress" ? 72 : 90)}`}>
          <input type="range" min={1} max={100} style={{ width: 220 }} value={options.quality ?? (info.action.kind === "image_compress" ? 72 : 90)} onChange={(e) => set({ quality: Number(e.target.value) })} />
        </Pair>
      )}
      {p.includes("resize") && (
        <>
          <Pair label="Size (px)" hint="Leave one empty to keep the aspect ratio">
            <Num value={options.width} onChange={(v) => set({ width: v, scale_pct: null })} placeholder="Width" min={1} max={16384} />
            <span className="faint">×</span>
            <Num value={options.height} onChange={(v) => set({ height: v, scale_pct: null })} placeholder="Height" min={1} max={16384} />
          </Pair>
          <Pair label="…or scale" hint="%">
            <Num value={options.scale_pct} onChange={(v) => set({ scale_pct: v, width: null, height: null })} placeholder="50" min={1} max={1000} />
          </Pair>
        </>
      )}
      {p.includes("crop") && (
        <Pair label="Crop area (px)" hint="x, y, width, height from the top-left corner">
          {(["x", "y", "width", "height"] as const).map((k) => (
            <Num
              key={k}
              value={options.crop?.[k]}
              placeholder={k}
              min={0}
              onChange={(v) => set({ crop: { x: 0, y: 0, width: 0, height: 0, ...options.crop, [k]: v ?? 0 } })}
            />
          ))}
        </Pair>
      )}
      {p.includes("rotate") && (
        <Pair label="Rotate">
          <div className="seg">
            {[0, 90, 180, 270].map((d) => (
              <button key={d} className={(options.rotate ?? 0) === d ? "on" : ""} onClick={() => set({ rotate: d })}>
                {d === 0 ? "None" : `${d}°`}
              </button>
            ))}
          </div>
          <label className="row small"><input type="checkbox" checked={!!options.flip_h} onChange={(e) => set({ flip_h: e.target.checked })} /> Flip horizontal</label>
          <label className="row small"><input type="checkbox" checked={!!options.flip_v} onChange={(e) => set({ flip_v: e.target.checked })} /> Flip vertical</label>
        </Pair>
      )}
      {(p.includes("resize") || p.includes("image_quality")) && info.group === "convert" && (
        <Pair label="Metadata">
          <label className="row small"><input type="checkbox" checked={!!options.strip_metadata} onChange={(e) => set({ strip_metadata: e.target.checked })} /> Remove EXIF / location data</label>
        </Pair>
      )}
      {p.includes("dpi") && (
        <Pair label="Resolution" hint="DPI (150 is good for screens, 300 for print)">
          <Num value={options.dpi} onChange={(v) => set({ dpi: v })} placeholder="150" min={36} max={1200} />
        </Pair>
      )}
      {p.includes("page_range") && (
        <Pair label="Pages" hint="e.g. 1-3,5,z (z = last). Empty = all">
          <input className="input" style={{ width: 160 }} value={options.pages ?? ""} placeholder="All pages" onChange={(e) => set({ pages: e.target.value || null })} />
        </Pair>
      )}
      {p.includes("page_order") && (
        <Pair label="Pages to keep, in order" hint="e.g. 3,1-2 reorders; 2-4 extracts">
          <input className="input" style={{ width: 200 }} value={options.pages ?? ""} placeholder="1-3,5" onChange={(e) => set({ pages: e.target.value || null })} />
        </Pair>
      )}
      {p.includes("pdf_rotate") && (
        <>
          <Pair label="Rotate by">
            <div className="seg">
              {[90, 180, 270].map((d) => (
                <button key={d} className={options.rotate === d ? "on" : ""} onClick={() => set({ rotate: d })}>{d}°</button>
              ))}
            </div>
          </Pair>
          <Pair label="Pages" hint="Empty = all pages">
            <input className="input" style={{ width: 160 }} value={options.pages ?? ""} placeholder="All pages" onChange={(e) => set({ pages: e.target.value || null })} />
          </Pair>
        </>
      )}
      {p.includes("password") && (
        <Pair
          label="Password"
          hint={
            info.action.kind === "pdf_unlock"
              ? "Only remove protection from files you are authorized to unlock."
              : info.action.kind === "archive_extract"
                ? "Only needed for encrypted archives"
                : info.action.kind === "archive_create"
                  ? "Optional; ZIP uses AES-256, 7z also hides file names"
                  : "AES-256"
          }
        >
          <input className="input" type="password" style={{ width: 200 }} value={options.password ?? ""} autoComplete="new-password" onChange={(e) => set({ password: e.target.value || null })} />
        </Pair>
      )}
      {p.includes("ocr_language") && (
        <Pair label="Language" hint={langs.length ? "" : "Tesseract languages installed on this PC"}>
          <select className="select" value={options.ocr_lang ?? "eng"} onChange={(e) => set({ ocr_lang: e.target.value })}>
            {(langs.length ? langs : ["eng"]).map((l) => (
              <option key={l} value={l}>{l}</option>
            ))}
          </select>
        </Pair>
      )}
      {p.includes("audio") && (
        <>
          {!(to && ["Wav", "Flac", "Aiff"].includes(to)) && (
            <Pair label="Bitrate">
              <select className="select" value={options.audio_bitrate_kbps ?? ""} onChange={(e) => set({ audio_bitrate_kbps: e.target.value ? Number(e.target.value) : null })}>
                <option value="">Automatic</option>
                {[64, 96, 128, 160, 192, 256, 320].map((b) => <option key={b} value={b}>{b} kbps</option>)}
              </select>
            </Pair>
          )}
          <Pair label="Sample rate">
            <select className="select" value={options.sample_rate ?? ""} onChange={(e) => set({ sample_rate: e.target.value ? Number(e.target.value) : null })}>
              <option value="">Keep</option>
              {[22050, 32000, 44100, 48000, 96000].map((s) => <option key={s} value={s}>{(s / 1000).toFixed(s % 1000 ? 2 : 0)} kHz</option>)}
            </select>
          </Pair>
          <Pair label="Channels">
            <div className="seg">
              {[[null, "Keep"], [1, "Mono"], [2, "Stereo"]].map(([v, l]) => (
                <button key={String(l)} className={(options.channels ?? null) === v ? "on" : ""} onClick={() => set({ channels: v as number | null })}>{l}</button>
              ))}
            </div>
          </Pair>
        </>
      )}
      {p.includes("video") && (
        <>
          {to && ["Mp4", "Mov", "Mkv", "Webm", "Avi"].includes(to) && (
            <Pair label="Codec">
              <select className="select" value={options.video_codec ?? ""} onChange={(e) => set({ video_codec: e.target.value || null })}>
                <option value="">Automatic</option>
                {to !== "Webm" && <option value="h264">H.264 (most compatible)</option>}
                {["Mp4", "Mov", "Mkv"].includes(to) && <option value="h265">H.265 / HEVC (smaller)</option>}
                {["Webm", "Mkv", "Mp4"].includes(to) && <option value="vp9">VP9</option>}
                {["Webm", "Mkv", "Mp4"].includes(to) && <option value="av1">AV1 (smallest, slow)</option>}
                {["Mp4", "Mov", "Mkv", "Avi"].includes(to) && <option value="mpeg4">MPEG-4 Part 2</option>}
                <option value="copy">Copy streams (no re-encode)</option>
              </select>
            </Pair>
          )}
          <Pair label="Quality" hint={`${options.crf ?? (info.action.kind === "video_compress" ? 28 : 23)} — lower is better, larger`}>
            <input type="range" min={14} max={40} style={{ width: 220 }} value={options.crf ?? (info.action.kind === "video_compress" ? 28 : 23)} onChange={(e) => set({ crf: Number(e.target.value) })} />
          </Pair>
          <Pair label="Resolution">
            <select className="select" value={options.resolution ?? ""} onChange={(e) => set({ resolution: e.target.value ? Number(e.target.value) : null })}>
              <option value="">Keep</option>
              {[2160, 1440, 1080, 720, 480, 360, 240].map((h) => <option key={h} value={h}>{h}p</option>)}
            </select>
          </Pair>
        </>
      )}
      {p.includes("fps") && (
        <Pair label="Frame rate" hint={info.action.kind === "extract_frames" ? "frames per second to export (default 1)" : "fps (empty = keep)"}>
          <Num value={options.fps} onChange={(v) => set({ fps: v })} placeholder={info.action.kind === "extract_frames" ? "1" : to === "Gif" ? "12" : "Keep"} min={0.1} max={240} step={0.5} />
        </Pair>
      )}
      {p.includes("gif_width") && (
        <Pair label="GIF width" hint="px">
          <Num value={options.gif_width} onChange={(v) => set({ gif_width: v })} placeholder="480" min={16} max={4096} />
        </Pair>
      )}
      {p.includes("trim_range") && (
        <Pair label="Trim" hint="seconds or mm:ss">
          <TimeInput value={options.start} onChange={(v) => set({ start: v })} placeholder="Start 0:00" />
          <span className="faint">→</span>
          <TimeInput value={options.end} onChange={(v) => set({ end: v })} placeholder="End" />
        </Pair>
      )}
    </div>
  );
}
