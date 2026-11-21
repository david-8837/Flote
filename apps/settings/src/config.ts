import { invoke } from "@tauri-apps/api/core";

export type Language = "auto" | "es" | "en" | "fr";
export type IslandStyle = "notch" | "bubble";
export type GenieSpeed = "normal" | "fast" | "faster";
export type DockPosition = "left" | "bottom" | "right";

/** Mirrors `muelle_config::Config`. */
export interface Config {
  version: number;
  pinned: string[];
  recent: string[];
  autoHide: boolean;
  iconSize: number;
  magnification: boolean;
  magnifiedSize: number;
  dockPosition: DockPosition;
  genie: boolean;
  genieSpeed: GenieSpeed;
  genieDurationMs: number | null;
  showStart: boolean;
  showDownloads: boolean;
  showRecycleBin: boolean;
  showMinimized: boolean;
  menuBar: boolean;
  menuBarAutoHide: boolean;
  aiUsage: boolean;
  menuBarMusic: boolean;
  menuBarIslandStyle: IslandStyle;
  accessoryBattery: boolean;
  menuBarSearch: boolean | null;
  menuBarTaskView: boolean | null;
  menuBarWidgets: boolean | null;
  menuBarBluetooth: boolean;
  menuBarBatteryPercent: boolean | null;
  showIndicators: boolean;
  showRecents: boolean;
  bounceOnOpen: boolean;
  mascot: boolean;
  checkUpdates: boolean;
  autoUpdate: boolean;
  language: Language;
  /** The folder shown at the right of the dock, whose contents open as a menu. */
  shortcutsFolder: string | null;
  /** Custom icons chosen by the user for apps or PWAs (`app_id` -> file path). */
  customIcons?: Record<string, string>;
}

export const ICON_SIZE = { min: 20, max: 96 } as const;
export const MAX_MAGNIFIED_SIZE = 160;
export const GENIE_DURATION_MS_RANGE = { min: 150, max: 1000 } as const;
export const DEFAULT_GENIE_DURATION_MS = 500;

export const loadConfig = () => invoke<Config>("get_config");
export const defaultConfig = () => invoke<Config>("default_config");
export const saveConfig = (patch: Partial<Config>) => invoke<Config>("set_config", { patch });
export const setExplorerPinned = (pinned: boolean) =>
  invoke<Config>("set_explorer_pinned", { pinned });
export const restoreDefaultTaskbarLayout = () =>
  invoke<Config>("restore_default_taskbar_layout");
export const pickIconFile = (current?: string | null) =>
  invoke<string | null>("pick_icon_file", { current: current ?? null });
export const setCustomIcon = (appId: string, iconPath: string | null) =>
  invoke<Config>("set_custom_icon", { appId, iconPath });
export const getAutostart = () => invoke<boolean>("get_autostart");
/** Windows' animation setting; the genie only plays while it is on. */
export const windowsAnimations = () => invoke<boolean>("windows_animations");
export const openAnimationSettings = () => invoke<void>("open_animation_settings");
export const setAutostart = (enabled: boolean) => invoke<void>("set_autostart", { enabled });
/** The folder the user picks, or null if they cancel. */
export const pickFolder = (current: string | null) =>
  invoke<string | null>("pick_folder", { current });
export interface UpdateInfo {
  version: string;
  /** Release page with the notes, if the manifest has one. */
  notes: string | null;
}

/** Resolves to the newer version available, or null when up to date. */
export const checkUpdate = () => invoke<UpdateInfo | null>("check_update");
export const openReleaseNotes = (url: string) => invoke<void>("open_release_notes", { url });
/** Fixed project URL, opened only when the user asks. */
export const openRepository = () => invoke<void>("open_repository");
let wallpaperRequest: Promise<string | null> | undefined;
/** One bounded native read per Settings process, including React StrictMode. */
export const loadWallpaperPreview = () =>
  wallpaperRequest ??= invoke<string | null>("wallpaper_preview").catch(() => null);
export const installUpdate = () => invoke<void>("install_update");
