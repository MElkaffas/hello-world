import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, type Action, type Category } from "../api";
import { CategoryIcon, Icon } from "../components/Icon";
import { errText, useStore } from "../store";
import { categoryLabels } from "../lib/util";

interface Tool { label: string; desc: string; action?: Action; metadata?: boolean }

const exts: Partial<Record<Category, string[]>> = {
  image: ["jpg", "jpeg", "png", "webp", "heic", "heif", "avif", "tif", "tiff", "bmp", "gif", "ico", "svg"],
  pdf: ["pdf"],
  video: ["mp4", "mov", "mkv", "avi", "webm", "wmv", "mpg", "mpeg", "m4v"],
  audio: ["mp3", "wav", "aac", "m4a", "flac", "ogg", "opus", "aiff", "aif", "wma"],
  document: ["docx", "doc", "odt", "rtf", "txt", "html", "htm", "md", "epub", "xlsx", "xls", "ods", "csv", "pptx", "ppt", "odp"],
  archive: ["zip", "rar", "7z", "tar", "gz", "tgz"],
};

const sections: { cat: Category; tools: Tool[] }[] = [
  {
    cat: "image",
    tools: [
      { label: "Compress", desc: "Smaller files, same format", action: { kind: "image_compress" } },
      { label: "Resize", desc: "Pixels or percentage", action: { kind: "image_resize" } },
      { label: "Crop", desc: "Keep a rectangle", action: { kind: "image_crop" } },
      { label: "Rotate / flip", desc: "90° steps, mirror", action: { kind: "image_rotate" } },
      { label: "Metadata viewer", desc: "Camera, EXIF, GPS", metadata: true },
      { label: "Remove metadata", desc: "Strip EXIF and location", action: { kind: "strip_metadata" } },
      { label: "Images to PDF", desc: "One page per image", action: { kind: "images_to_pdf" } },
      { label: "GIF frames", desc: "Every frame as PNG", action: { kind: "gif_frames" } },
      { label: "Image OCR", desc: "Text or searchable PDF", action: { kind: "image_ocr", to: "Pdf" } },
    ],
  },
  {
    cat: "pdf",
    tools: [
      { label: "Merge", desc: "Combine PDFs in order", action: { kind: "pdf_merge" } },
      { label: "Split", desc: "One PDF per page", action: { kind: "pdf_split" } },
      { label: "Optimize size", desc: "Lossless stream compression", action: { kind: "pdf_compress" } },
      { label: "Extract / reorder pages", desc: "e.g. 3,1-2", action: { kind: "pdf_pages" } },
      { label: "Rotate pages", desc: "All or selected pages", action: { kind: "pdf_rotate" } },
      { label: "OCR", desc: "Searchable PDF from scans", action: { kind: "pdf_ocr" } },
      { label: "PDF to images", desc: "JPG / PNG / WebP per page", action: { kind: "convert", to: "Png" } },
      { label: "Protect", desc: "AES-256 password", action: { kind: "pdf_protect" } },
      { label: "Remove password", desc: "When you know the password", action: { kind: "pdf_unlock" } },
    ],
  },
  {
    cat: "video",
    tools: [
      { label: "Compress", desc: "H.264, adjustable quality", action: { kind: "video_compress" } },
      { label: "Trim", desc: "Start and end time", action: { kind: "trim" } },
      { label: "Crop", desc: "Keep a region", action: { kind: "video_crop" } },
      { label: "Extract audio", desc: "MP3 soundtrack", action: { kind: "convert", to: "Mp3" } },
      { label: "Extract frames", desc: "PNG every N seconds", action: { kind: "extract_frames" } },
      { label: "Change resolution", desc: "720p, 1080p, FPS", action: { kind: "video_resize" } },
      { label: "Video to GIF", desc: "Palette-optimized", action: { kind: "convert", to: "Gif" } },
      { label: "Merge", desc: "Join clips", action: { kind: "media_merge" } },
    ],
  },
  {
    cat: "audio",
    tools: [
      { label: "Convert", desc: "MP3, AAC, FLAC, Opus…", action: { kind: "convert", to: "Mp3" } },
      { label: "Compress", desc: "Lower bitrate", action: { kind: "audio_compress" } },
      { label: "Trim", desc: "Start and end time", action: { kind: "trim" } },
      { label: "Merge", desc: "Join tracks", action: { kind: "media_merge" } },
      { label: "Normalize", desc: "EBU R128 loudness", action: { kind: "normalize" } },
    ],
  },
  {
    cat: "document",
    tools: [
      { label: "Export to PDF", desc: "Word, Excel, PowerPoint, Markdown", action: { kind: "convert", to: "Pdf" } },
      { label: "Convert", desc: "DOCX, ODT, RTF, HTML, MD, EPUB" },
      { label: "Extract text", desc: "Plain UTF-8 text", action: { kind: "convert", to: "Txt" } },
      { label: "Slides to images", desc: "One PNG per slide", action: { kind: "slides_to_images", to: "Png" } },
    ],
  },
  {
    cat: "archive",
    tools: [
      { label: "Extract", desc: "ZIP, RAR, 7Z, TAR, GZ", action: { kind: "archive_extract" } },
      { label: "Create ZIP", desc: "Optional AES-256 password", action: { kind: "archive_create", to: "Zip" } },
      { label: "Create 7z", desc: "Best compression", action: { kind: "archive_create", to: "SevenZ" } },
      { label: "Convert archive", desc: "e.g. RAR → ZIP", action: { kind: "convert", to: "Zip" } },
    ],
  },
];

export function ToolsView({ pickFiles }: { pickFiles: (f?: { name: string; extensions: string[] }[]) => Promise<string[]> }) {
  const { setPreselect, toast } = useStore();
  const [meta, setMeta] = useState<{ name: string; rows: [string, string][] } | null>(null);

  const start = async (cat: Category, t: Tool) => {
    const filters = exts[cat] ? [{ name: categoryLabels[cat], extensions: exts[cat]! }] : undefined;
    if (t.metadata) {
      const sel = await open({ multiple: false, filters });
      if (typeof sel !== "string") return;
      try {
        setMeta({ name: sel, rows: await api.imageMetadata(sel) });
      } catch (e) {
        toast("err", errText(e));
      }
      return;
    }
    setPreselect(t.action ?? null);
    const picked = await pickFiles(filters);
    if (!picked.length) setPreselect(null);
  };

  return (
    <div className="page">
      <div className="page-head">
        <div>
          <h1>Tools</h1>
          <p>Pick a tool, choose files, adjust settings. Only tools that fit the chosen files are offered in the workspace.</p>
        </div>
      </div>
      {sections.map((s) => (
        <div key={s.cat}>
          <h2 className="row"><CategoryIcon category={s.cat} size={24} /> {categoryLabels[s.cat]}</h2>
          <div className="grid c3">
            {s.tools.map((t) => (
              <button key={t.label} className="tile" onClick={() => start(s.cat, t)}>
                <div className="grow">
                  <div className="t-main">{t.label}</div>
                  <div className="t-sub">{t.desc}</div>
                </div>
                <Icon name="arrow" size={15} />
              </button>
            ))}
          </div>
        </div>
      ))}

      {meta && (
        <div className="modal-back" onMouseDown={() => setMeta(null)}>
          <div className="modal card card-pad" onMouseDown={(e) => e.stopPropagation()}>
            <div className="row" style={{ marginBottom: 12 }}>
              <b className="grow ellipsis">{meta.name.split(/[\\/]/).pop()}</b>
              <button className="icon-btn" onClick={() => setMeta(null)}><Icon name="x" size={16} /></button>
            </div>
            {meta.rows.length === 0 ? (
              <div className="faint">No metadata found.</div>
            ) : (
              <table className="tbl selectable">
                <tbody>
                  {meta.rows.map(([k, v], i) => (
                    <tr key={i}><td className="small faint" style={{ width: 200 }}>{k}</td><td className="small mono" style={{ wordBreak: "break-all" }}>{v}</td></tr>
                  ))}
                </tbody>
              </table>
            )}
            <div className="small faint" style={{ marginTop: 12 }}>To remove this information, use Images → Remove metadata.</div>
          </div>
        </div>
      )}
    </div>
  );
}
