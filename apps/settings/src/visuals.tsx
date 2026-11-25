import { useEffect, useRef, type CSSProperties, type ReactNode } from "react";
import type { DockPosition, GenieSpeed, IslandStyle } from "./config";

const PATHS = {
  size: <><rect x="3" y="9" width="6" height="6" rx="1.5" /><rect x="13" y="5" width="9" height="14" rx="2" /></>,
  magnification: <><circle cx="10" cy="10" r="6" /><path d="m15 15 5 5M7 10h6m-3-3v6" /></>,
  hide: <><path d="M3 9V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v4M3 15v4a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-4M8 12h8m-3-3 3 3-3 3" /></>,
  bounce: <><rect x="8" y="9" width="8" height="8" rx="2" /><path d="m9 5 3-3 3 3M12 2v5M6 21h12" /></>,
  mascot: <><path d="M6 13a6 6 0 0 0 12 0v-2H6zM8 11V7m8 4V7M4 9l-2-3m18 3 2-3M6 13H3m15 0h3M6 16l-3 3m15-3 3 3" /><circle cx="8" cy="6" r="1" /><circle cx="16" cy="6" r="1" /></>,
  indicators: <><rect x="3" y="4" width="7" height="10" rx="2" /><rect x="14" y="4" width="7" height="10" rx="2" /><circle cx="6.5" cy="19" r="1" fill="currentColor" /><circle cx="17.5" cy="19" r="1" fill="currentColor" /></>,
  recents: <><path d="M4 10a8 8 0 1 1 1 8M4 4v6h6m2-3v5l3 2" /></>,
  folder: <path d="M3 7V5a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7Z" />,
  windows: <><rect x="3" y="3" width="13" height="11" rx="2" /><path d="M8 17v2a2 2 0 0 0 2 2h9a2 2 0 0 0 2-2v-9a2 2 0 0 0-2-2" /><path d="M6 7h7" /></>,
  genie: <><path d="M4 4h16v9c0 4-8 3-8 7M4 4v9c0 4 8 3 8 7M9 21h6M7 8h10" /></>,
  menu: <><rect x="3" y="4" width="18" height="16" rx="2" /><path d="M3 9h18m-5-3h2M6 6h1" /></>,
  notch: <><rect x="3" y="3" width="18" height="18" rx="3" /><path d="M7 3h10v2a3 3 0 0 1-3 3h-4a3 3 0 0 1-3-3V3z" fill="currentColor" /></>,
  music: <><path d="M9 17V5l12-2v12M9 9l12-2" /><ellipse cx="6" cy="18" rx="3" ry="2" /><ellipse cx="18" cy="16" rx="3" ry="2" /></>,
  ai: <><path d="m12 2 2.6 7.4L22 12l-7.4 2.6L12 22l-2.6-7.4L2 12l7.4-2.6z" /></>,
  headphones: <><path d="M4 14v-3a8 8 0 0 1 16 0v3" /><rect x="3" y="12" width="4" height="8" rx="2" /><rect x="17" y="12" width="4" height="8" rx="2" /></>,
  bluetooth: <path d="m7 7 10 10-5 4V3l5 4L7 17" />,
  battery: <><rect x="2" y="6" width="18" height="12" rx="2" /><path d="M23 10v4M6 10v4m4-4v4m4-4v4" /></>,
  search: <><circle cx="10" cy="10" r="7" /><path d="m15 15 6 6" /></>,
  widgets: <><rect x="3" y="3" width="7" height="18" rx="2" /><rect x="14" y="3" width="7" height="7" rx="2" /><rect x="14" y="14" width="7" height="7" rx="2" /></>,
  start: <><path d="M12 3v9M6 5a9 9 0 1 0 12 0" /></>,
  language: <><circle cx="12" cy="12" r="10" /><ellipse cx="12" cy="12" rx="4" ry="10" /><path d="M2 12h20" /></>,
  updates: <><path d="M20 8a8 8 0 0 0-14-3L3 8m0-5v5h5m-4 8a8 8 0 0 0 14 3l3-3m-5 0h5v5" /></>,
  reset: <><path d="M3 10a9 9 0 1 1 1 8M3 4v6h6" /></>,
  star: <path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1 6.2-5.5-2.9-5.5 2.9 1-6.2L3 9.6l6.2-.9z" />,
  external: <><path d="M14 3h7v7m0-7L10 14M10 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-5" /></>,
} satisfies Record<string, ReactNode>;

export function SettingIcon({ name, size = 18 }: { name: keyof typeof PATHS; size?: number }) {
  return <svg viewBox="0 0 24 24" width={size} height={size} fill={name === "star" ? "#f5c344" : "none"} stroke={name === "star" ? "#d69b16" : "currentColor"} strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">{PATHS[name]}</svg>;
}

const MINI_ICONS = <><i /><i /><i /><i /><i /></>;

/** Illustrations only: no window capture, network, timers or animation loop. */
export function DockPreview({ position, iconSize, wallpaper }: { position: DockPosition; iconSize: number; wallpaper: string | null }) {
  return <span className="preview-screen">
    {wallpaper ? <img className="preview-wallpaper" src={wallpaper} alt="" /> : null}
    <span className="preview-topbar"><i /><i /><i /></span>
    <span className="preview-window"><i /><i /><i /></span>
    <span className="preview-dock" data-position={position}
      style={{ "--mini-icon": `${Math.max(4, Math.round((5 + (iconSize - 20) * (5 / 76)) * 10) / 10)}px` } as CSSProperties}>{MINI_ICONS}</span>
  </span>;
}

export function IslandPreview({ style, wallpaper }: { style: IslandStyle; wallpaper: string | null }) {
  return <span className="preview-screen preview-screen-island">
    {wallpaper ? <img className="preview-wallpaper" src={wallpaper} alt="" /> : null}
    <span className="preview-topbar"><i /><i /><i /></span>
    <span className="preview-island" data-style={style}>
      {style === "notch" ? <svg viewBox="0 0 100 24"><path d="M0 0h100c-12 0-8 24-24 24H24C8 24 12 0 0 0" fill="currentColor" /></svg> : null}
      <span className="preview-island-content">
        <span className="preview-album"><svg viewBox="0 0 16 16"><path d="M6 11V4l6-1v7M6 6l6-1" /><ellipse cx="4" cy="12" rx="2" ry="1.5" /><ellipse cx="10" cy="11" rx="2" ry="1.5" /></svg></span>
        <span className="preview-song"><i /><i /></span>
        <span className="preview-wave"><i /><i /><i /></span>
      </span>
    </span>
    <span className="preview-window"><i /><i /><i /></span>
  </span>;
}

export function SpeedPreview({ speed }: { speed: GenieSpeed }) {
  return <svg className="preview-speed" viewBox="0 0 120 48" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <rect x="20" y="4" width="36" height="24" rx="4" opacity=".65" /><path d="M20 11h36" opacity=".35" />
    <path d="M63 16c17 0 7 23 23 23m-5-4 5 4-5 3" stroke="var(--accent)" />
    {speed !== "normal" ? <path d="M12 12H5m7 6H2m10 6H5" opacity=".4" /> : null}
    {speed === "faster" ? <path d="M8 6H3m5 24H3" opacity=".25" /> : null}
    <rect x="88" y="34" width="16" height="10" rx="2" fill="var(--accent)" stroke="none" />
  </svg>;
}

function drawRoundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  if (typeof ctx.roundRect === "function") {
    ctx.roundRect(x, y, w, h, r);
  } else {
    ctx.rect(x, y, w, h);
  }
}

/** Smooth real-time canvas preview of the Genie minimize and restore effect. */
export function GenieSpeedPreview({ durationMs }: { durationMs: number }) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let animId: number;
    let stateStartTime = performance.now();
    let phase: "open" | "minimizing" | "minimized" | "restoring" = "open";
    const PAUSE_MS = 350;

    const render = (now: number) => {
      if (document.hidden) {
        animId = requestAnimationFrame(render);
        return;
      }

      const elapsed = now - stateStartTime;
      let t = 0;

      if (phase === "open") {
        t = 0;
        if (elapsed >= PAUSE_MS) {
          phase = "minimizing";
          stateStartTime = now;
        }
      } else if (phase === "minimizing") {
        const progress = Math.min(1, elapsed / Math.max(1, durationMs));
        t = progress;
        if (progress >= 1) {
          phase = "minimized";
          stateStartTime = now;
        }
      } else if (phase === "minimized") {
        t = 1;
        if (elapsed >= PAUSE_MS) {
          phase = "restoring";
          stateStartTime = now;
        }
      } else if (phase === "restoring") {
        const progress = Math.min(1, elapsed / Math.max(1, durationMs));
        t = 1 - progress;
        if (progress >= 1) {
          phase = "open";
          stateStartTime = now;
        }
      }

      const dpr = window.devicePixelRatio || 1;
      const cssWidth = canvas.clientWidth || 240;
      const cssHeight = canvas.clientHeight || 96;

      if (canvas.width !== cssWidth * dpr || canvas.height !== cssHeight * dpr) {
        canvas.width = cssWidth * dpr;
        canvas.height = cssHeight * dpr;
      }

      ctx.save();
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, cssWidth, cssHeight);

      // Desktop mockup background
      const isDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
      ctx.fillStyle = isDark ? "rgba(255, 255, 255, 0.04)" : "rgba(0, 0, 0, 0.03)";
      ctx.beginPath();
      drawRoundRect(ctx, 0, 0, cssWidth, cssHeight, 8);
      ctx.fill();

      // Mini dock at bottom
      const dockWidth = Math.min(180, cssWidth * 0.75);
      const dockHeight = 16;
      const dockX = (cssWidth - dockWidth) / 2;
      const dockY = cssHeight - dockHeight - 6;

      ctx.fillStyle = isDark ? "rgba(255, 255, 255, 0.12)" : "rgba(0, 0, 0, 0.08)";
      ctx.beginPath();
      drawRoundRect(ctx, dockX, dockY, dockWidth, dockHeight, 6);
      ctx.fill();

      // Slot target icon on dock
      const slotX = dockX + dockWidth * 0.62;
      const slotY = dockY + 2.5;
      const slotWidth = 11;
      const slotHeight = 11;

      // Other dock icons
      ctx.fillStyle = isDark ? "rgba(255, 255, 255, 0.3)" : "rgba(0, 0, 0, 0.2)";
      for (let i = 0; i < 6; i++) {
        const ix = dockX + 8 + i * 15;
        if (Math.abs(ix - slotX) > 8) {
          ctx.beginPath();
          drawRoundRect(ctx, ix, dockY + 3, 10, 10, 2);
          ctx.fill();
        }
      }

      // Slot icon (active app icon)
      ctx.fillStyle = "rgba(10, 108, 240, 0.85)";
      ctx.beginPath();
      drawRoundRect(ctx, slotX, slotY, slotWidth, slotHeight, 2.5);
      ctx.fill();

      // Window flying or resting open
      if (t < 0.99) {
        const winLeft = cssWidth * 0.22;
        const winRight = cssWidth * 0.52;
        const winTop = 8;
        const winBottom = cssHeight * 0.58;
        const winHeight = winBottom - winTop;

        const count = 30;
        const bendEnd = 0.5;
        const slideStart = 0.15;

        const smoothstep = (x: number) => {
          const c = Math.max(0, Math.min(1, x));
          return c * c * (3 - 2 * c);
        };

        const bend = smoothstep(t / bendEnd);
        const slideT = Math.max(0, Math.min(1, (t - slideStart) / (1 - slideStart)));
        const shift = slideT * slideT * Math.max(0, slotY + slotHeight - winTop);
        const span = slotY - winTop;

        const sides = (y: number) => {
          const along = span > 0 ? Math.max(0, Math.min(1, (y - winTop) / span)) : 1;
          const curve = ((1 - Math.cos(Math.PI * along)) / 2) * bend;
          return [
            winLeft + (slotX - winLeft) * curve,
            winRight + (slotX + slotWidth - winRight) * curve,
          ];
        };

        for (let i = 0; i < count; i++) {
          const top = winTop + (i / count) * winHeight + shift;
          if (top >= slotY + slotHeight) break;
          const bottom = Math.min(winTop + ((i + 1) / count) * winHeight + shift, slotY + slotHeight);
          const [left, right] = sides((top + bottom) / 2);
          const stripW = Math.max(1, right - left);
          const stripH = Math.max(0.6, bottom - top + 0.6);

          // Top strips represent the titlebar
          if (i < 4) {
            ctx.fillStyle = isDark ? "rgba(90, 150, 240, 0.9)" : "rgba(10, 108, 240, 0.9)";
          } else {
            ctx.fillStyle = isDark ? "rgba(60, 65, 80, 0.85)" : "rgba(235, 240, 250, 0.85)";
          }
          ctx.fillRect(left, top, stripW, stripH);
        }
      }

      ctx.restore();
      animId = requestAnimationFrame(render);
    };

    animId = requestAnimationFrame(render);
    return () => cancelAnimationFrame(animId);
  }, [durationMs]);

  return (
    <div className="genie-preview-container">
      <canvas ref={canvasRef} className="genie-preview-canvas" aria-hidden="true" />
    </div>
  );
}
