// Small stroke icon set (original drawings, 24px grid).
const paths: Record<string, string> = {
  home: "M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z",
  workspace: "M4 5h16v14H4zM4 9h16M9 9v10",
  jobs: "M12 3v4M12 17v4M4.9 4.9l2.8 2.8M16.3 16.3l2.8 2.8M3 12h4M17 12h4M4.9 19.1l2.8-2.8M16.3 7.7l2.8-2.8",
  history: "M3 12a9 9 0 1 0 3-6.7L3 8M3 3v5h5M12 7v5l3 2",
  tools: "M14.7 6.3a4 4 0 0 0-5.4 5.4L3 18l3 3 6.3-6.3a4 4 0 0 0 5.4-5.4l-2.5 2.5-2.4-.6-.6-2.4z",
  settings: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z",
  info: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 16v-4M12 8h.01",
  upload: "M12 16V4M7 9l5-5 5 5M4 16v3a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-3",
  file: "M14 3H6a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8zM14 3v5h5",
  folder: "M3 6a1 1 0 0 1 1-1h5l2 2h9a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z",
  image: "M4 5h16v14H4zM4 16l4.5-4.5 3.5 3.5 3-3 5 5M15.5 9.5h.01",
  pdf: "M14 3H6a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8zM14 3v5h5M8.5 15h7M8.5 12h7",
  audio: "M9 18V6l11-2v12M9 18a3 3 0 1 1-6 0 3 3 0 0 1 6 0zM20 16a3 3 0 1 1-6 0 3 3 0 0 1 6 0z",
  video: "M3 6h13v12H3zM16 10l5-3v10l-5-3",
  document: "M14 3H6a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8zM14 3v5h5M8.5 13h7M8.5 16.5h5",
  spreadsheet: "M4 4h16v16H4zM4 10h16M4 15h16M10 4v16",
  presentation: "M3 4h18M5 4v10h14V4M12 14v4M8 21l4-3 4 3",
  archive: "M4 7h16v13H4zM3 3h18v4H3zM10 11h4",
  x: "M6 6l12 12M18 6 6 18",
  check: "M5 12.5 10 17l9-10",
  star: "M12 3.5l2.6 5.3 5.9.9-4.3 4.1 1 5.8L12 16.9l-5.2 2.7 1-5.8-4.3-4.1 5.9-.9z",
  search: "M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM20 20l-4-4",
  reveal: "M3 6a1 1 0 0 1 1-1h5l2 2h9a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1zM12 11v5M9.5 13.5 12 11l2.5 2.5",
  open: "M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5",
  refresh: "M20 11a8 8 0 0 0-14.9-3.9L3 9M3 4v5h5M4 13a8 8 0 0 0 14.9 3.9L21 15M21 20v-5h-5",
  pause: "M8 5v14M16 5v14",
  play: "M7 4v16l13-8z",
  trash: "M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3",
  arrow: "M5 12h14M13 6l6 6-6 6",
  shield: "M12 3 5 6v6c0 4.4 3 7.8 7 9 4-1.2 7-4.6 7-9V6z",
  bolt: "M13 2 4 14h7l-1 8 9-12h-7z",
  drop: "M12 3.5c3 4 6 7.2 6 10.5a6 6 0 0 1-12 0c0-3.3 3-6.5 6-10.5z",
  plus: "M12 5v14M5 12h14",
  alert: "M12 3 2 20h20zM12 10v4M12 17h.01",
  repeat: "M17 2l4 4-4 4M3 11V9a3 3 0 0 1 3-3h15M7 22l-4-4 4-4M21 13v2a3 3 0 0 1-3 3H3",
  wheel: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM12 3v6M12 15v6M3 12h6M15 12h6",
};

export function Icon({ name, size = 18, stroke = 1.8, fill = false }: { name: string; size?: number; stroke?: number; fill?: boolean }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill={fill ? "currentColor" : "none"} stroke="currentColor" strokeWidth={stroke} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={paths[name] ?? paths.file} />
    </svg>
  );
}

export function CategoryIcon({ category, size = 34 }: { category: string | null; size?: number }) {
  const c = category ?? "folder";
  const icon = c === "folder" ? "folder" : c;
  return (
    <div className={`t-icon cat-${c}`} style={{ width: size, height: size, borderRadius: size * 0.27, display: "grid", placeItems: "center", flex: "none" }}>
      <Icon name={icon} size={size * 0.52} />
    </div>
  );
}
