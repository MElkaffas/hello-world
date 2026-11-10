import { useEffect, useMemo, useState } from "react";
import type { ActionInfo } from "../api";
import { requiredParams } from "../lib/util";

/**
 * Radial action wheel shown where files were dropped. Two rings: Convert and Tools (Tab
 * switches). Keyboard: arrows move, Enter runs, 1–8 run directly, Esc closes.
 */
export function RadialMenu(props: {
  x: number;
  y: number;
  actions: ActionInfo[];
  count: number;
  loading?: boolean;
  size?: number;
  onPick: (a: ActionInfo) => void;
  onMore: () => void;
  onClose: () => void;
}) {
  const [mode, setMode] = useState<"convert" | "tool">("convert");
  const [hot, setHot] = useState<number | null>(null);

  const sets = useMemo(() => {
    const conv = props.actions.filter((a) => a.group === "convert");
    const sorted = [...conv.filter((a) => a.recommended), ...conv.filter((a) => !a.recommended)].slice(0, 8);
    const tools = props.actions.filter((a) => a.group === "tool" && !a.params.some((p) => requiredParams.includes(p)) && !a.action.kind.startsWith("archive_create")).slice(0, 7);
    const zip = props.actions.find((a) => a.action.kind === "archive_create" && a.action.to === "Zip");
    if (zip) tools.push(zip);
    return { convert: sorted, tool: tools };
  }, [props.actions]);

  useEffect(() => {
    if (sets.convert.length === 0 && sets.tool.length > 0) setMode("tool");
  }, [sets]);

  const items = sets[mode];

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") props.onClose();
      else if (e.key === "Tab") {
        e.preventDefault();
        setMode((m) => (m === "convert" ? "tool" : "convert"));
        setHot(null);
      } else if (e.key === "ArrowRight" || e.key === "ArrowDown") setHot((h) => (h == null ? 0 : (h + 1) % items.length));
      else if (e.key === "ArrowLeft" || e.key === "ArrowUp") setHot((h) => (h == null ? items.length - 1 : (h - 1 + items.length) % items.length));
      else if (e.key === "Enter" && hot != null && items[hot]) props.onPick(items[hot]);
      else if (/^[1-8]$/.test(e.key)) {
        const i = parseInt(e.key, 10) - 1;
        if (items[i]) props.onPick(items[i]);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [items, hot, props]);

  const S = 340;
  const c = S / 2;
  const R = 160;
  const r = 62;
  const n = Math.max(items.length, 1);
  const step = (Math.PI * 2) / n;
  const gap = n > 1 ? 0.012 : 0;

  const wedge = (i: number) => {
    const a0 = -Math.PI / 2 - step / 2 + i * step + gap;
    const a1 = a0 + step - gap * 2;
    const p = (rad: number, a: number) => `${c + rad * Math.cos(a)},${c + rad * Math.sin(a)}`;
    const large = step - gap * 2 > Math.PI ? 1 : 0;
    if (n === 1) {
      return `M ${c - R},${c} A ${R},${R} 0 1 1 ${c + R},${c} A ${R},${R} 0 1 1 ${c - R},${c} M ${c - r},${c} A ${r},${r} 0 1 0 ${c + r},${c} A ${r},${r} 0 1 0 ${c - r},${c} Z`;
    }
    return `M ${p(R, a0)} A ${R},${R} 0 ${large} 1 ${p(R, a1)} L ${p(r, a1)} A ${r},${r} 0 ${large} 0 ${p(r, a0)} Z`;
  };
  const labelPos = (i: number) => {
    const a = -Math.PI / 2 + i * step;
    const rad = (R + r) / 2;
    return { x: c + rad * Math.cos(a), y: c + rad * Math.sin(a) };
  };
  const short = (a: ActionInfo) => {
    if (a.group === "convert") {
      const t = a.label.replace(/ \(.*\)$/, "");
      return t.length > 12 ? t.slice(0, 11) + "…" : t;
    }
    return a.label.length > 13 ? a.label.split(" ").slice(0, 2).join(" ") : a.label;
  };

  return (
    <>
      <div className="wheel-backdrop" onMouseDown={props.onClose} />
      <div className="wheel" style={{ left: props.x, top: props.y, width: props.size ?? S, height: props.size ?? S }} role="menu" aria-label="Choose an action">
        <svg viewBox={`0 0 ${S} ${S}`}>
          {items.map((a, i) => {
            const lp = labelPos(i);
            const isHot = hot === i;
            return (
              <g key={`${mode}-${i}`} onMouseEnter={() => setHot(i)} onMouseLeave={() => setHot(null)} onClick={() => props.onPick(a)} role="menuitem" aria-label={a.label}>
                <path className={`wedge ${isHot ? "hot" : ""}`} d={wedge(i)} />
                <text className={`wedge-label ${isHot ? "hot" : ""}`} x={lp.x} y={lp.y + (a.recommended ? -1 : 4)} textAnchor="middle">
                  {short(a)}
                </text>
                {a.recommended && (
                  <text className={`wedge-sub ${isHot ? "hot" : ""}`} x={lp.x} y={lp.y + 13} textAnchor="middle">
                    recommended
                  </text>
                )}
                <text className="wedge-num" x={labelPos(i).x + (R - r) * 0.0} y={lp.y - 17} textAnchor="middle">{i + 1}</text>
              </g>
            );
          })}
          {items.length === 0 && (
            <text className="hub-sub" x={c} y={c - 80} textAnchor="middle">{props.loading ? "Reading files…" : "No options"}</text>
          )}
          <g onClick={props.onMore} role="button" aria-label="More options">
            <circle className="hub" cx={c} cy={c} r={r - 6} />
            <text className="hub-text" x={c} y={c - 6} textAnchor="middle">{mode === "convert" ? "Convert" : "Tools"}</text>
            <text className="hub-sub" x={c} y={c + 9} textAnchor="middle">{props.count} file{props.count === 1 ? "" : "s"}</text>
            <text className="hub-sub" x={c} y={c + 23} textAnchor="middle">More…</text>
          </g>
        </svg>
        <div style={{ position: "absolute", left: "50%", bottom: -34, transform: "translateX(-50%)", whiteSpace: "nowrap" }} className="small faint">
          Tab: {mode === "convert" ? "tools" : "convert"} · 1–{Math.max(items.length, 1)} · Esc
        </div>
      </div>
    </>
  );
}
