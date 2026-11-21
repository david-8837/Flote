import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  type Config,
  type DockPosition,
  type GenieSpeed,
  type IslandStyle,
  type Language,
  type UpdateInfo,
  DEFAULT_GENIE_DURATION_MS,
  GENIE_DURATION_MS_RANGE,
  ICON_SIZE,
  MAX_MAGNIFIED_SIZE,
  checkUpdate,
  defaultConfig,
  getAutostart,
  openAnimationSettings,
  windowsAnimations,
  installUpdate,
  loadConfig,
  loadWallpaperPreview,
  openReleaseNotes,
  openRepository,
  pickFolder,
  pickIconFile,
  restoreDefaultTaskbarLayout,
  saveConfig,
  setAutostart,
  setCustomIcon,
  setExplorerPinned,
} from "./config";
import { Card, Row, Segmented, Slider, SubRow, Toggle, VisualChoices } from "./controls";
import { DockPreview, IslandPreview, SettingIcon, GenieSpeedPreview } from "./visuals";
import { type SectionId, SectionIcon } from "./icons";
import { UpdateBanner } from "./UpdateBanner";
import en from "./i18n/en.json";
import es from "./i18n/es.json";
import fr from "./i18n/fr.json";

type UpdateState =
  | { kind: "off" }
  | { kind: "checking" }
  | { kind: "current" }
  | { kind: "available"; info: UpdateInfo }
  | { kind: "installing"; info: UpdateInfo; progress: number | null }
  | { kind: "installFailed"; info: UpdateInfo }
  | { kind: "error" };

const toUpdateState = (found: UpdateInfo | null): UpdateState =>
  found ? { kind: "available", info: found } : { kind: "current" };

/** An update is waiting for the user: the banner and the badge show. */
const pendingUpdate = (state: UpdateState): UpdateInfo | null =>
  state.kind === "available" || state.kind === "installing" || state.kind === "installFailed"
    ? state.info
    : null;

interface ProgressEvent {
  downloaded: number;
  total: number | null;
}
type Texts = typeof es;

/** Dragging a slider saves at most this often; the dock redraws on each save. */
const SAVE_DELAY_MS = 60;
/** How long the reset button stays armed after the first click. */
const RESET_CONFIRM_MS = 4000;

const TEXTS: Record<Exclude<Language, "auto">, Texts> = { es, en, fr };

function textsFor(language: Language): Texts {
  if (language !== "auto") {
    return TEXTS[language];
  }
  // Follow the Windows display language, English when there is no text in it.
  const windows = navigator.language.toLowerCase().slice(0, 2);
  return windows === "es" || windows === "fr" ? TEXTS[windows] : en;
}

export function App() {
  const [config, setConfig] = useState<Config | null>(null);
  const [autostart, setAutostartState] = useState(false);
  // Assumed on until asked, so the notice never flashes on a normal PC.
  const [animations, setAnimations] = useState(true);
  useEffect(() => {
    const check = () => {
      windowsAnimations()
        .then(setAnimations)
        .catch(() => {});
    };
    check();
    // Back from Windows Settings after turning them on: the notice goes.
    window.addEventListener("focus", check);
    return () => window.removeEventListener("focus", check);
  }, []);
  const [section, setSection] = useState<SectionId>("dock");
  const [failed, setFailed] = useState(false);
  const [version, setVersion] = useState("");
  const [updateState, setUpdateState] = useState<UpdateState>({ kind: "checking" });
  const [confirmingReset, setConfirmingReset] = useState(false);
  const [projectError, setProjectError] = useState(false);
  const [wallpaper, setWallpaper] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    loadWallpaperPreview().then((image) => { if (alive) setWallpaper(image); });
    return () => { alive = false; };
  }, []);
  /** "Later" hides the banner until Settings is opened again. */
  const [bannerDismissed, setBannerDismissed] = useState(false);
  const pending = useRef<Partial<Config> | null>(null);
  const writes = useRef(Promise.resolve());
  const timer = useRef<number | undefined>(undefined);
  const resetTimer = useRef<number | undefined>(undefined);

  const flushSettings = useCallback(() => {
    window.clearTimeout(timer.current);
    const latest = pending.current;
    pending.current = null;
    if (latest) {
      writes.current = writes.current
        .then(() => saveConfig(latest))
        .then(() => setFailed(false))
        .catch(() => setFailed(true));
    }
    return writes.current;
  }, []);

  useEffect(() => {
    let alive = true;
    Promise.all([loadConfig(), getAutostart()])
      .then(([loaded, starts]) => {
        if (!alive) return;
        setConfig(loaded);
        setAutostartState(starts);
        // The only network access in Muelle, and the user can turn it off.
        if (!loaded.checkUpdates) {
          setUpdateState({ kind: "off" });
          return;
        }
        checkUpdate()
          .then((found) => alive && setUpdateState(toUpdateState(found)))
          .catch(() => alive && setUpdateState({ kind: "error" }));
      })
      .catch(() => alive && setFailed(true));
    getVersion()
      .then((current) => alive && setVersion(current))
      .catch(() => {});
    const progress = listen<ProgressEvent>("update-progress", ({ payload }) => {
      const share = payload.total ? Math.min(payload.downloaded / payload.total, 1) : null;
      setUpdateState((state) => (state.kind === "installing" ? { ...state, progress: share } : state));
    });
    let closing = false;
    const close = getCurrentWindow().onCloseRequested(async (event) => {
      if (!alive || closing) {
        event.preventDefault();
        return;
      }
      closing = true;
      // A change can be buffered or queued while an earlier write finishes.
      let drained: Promise<void>;
      do {
        drained = flushSettings();
        await drained;
      } while (pending.current || writes.current !== drained);
    });
    return () => {
      alive = false;
      window.clearTimeout(timer.current);
      window.clearTimeout(resetTimer.current);
      progress.then((unlisten) => unlisten()).catch(() => {});
      close.then((unlisten) => unlisten()).catch(() => {});
    };
  }, [flushSettings]);

  // Shows the change at once and saves the latest value shortly after, so a
  // slider drag does not write the file on every pixel.
  const update = useCallback((patch: Partial<Config>) => {
    setConfig((current) => {
      if (!current) return current;
      const next = { ...current, ...patch };
      if (next.magnifiedSize < next.iconSize) next.magnifiedSize = next.iconSize;
      return next;
    });
    pending.current = { ...pending.current, ...patch };
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => void flushSettings(), SAVE_DELAY_MS);
  }, [flushSettings]);

  const changeAutostart = useCallback((enabled: boolean) => {
    setAutostartState(enabled);
    setAutostart(enabled).catch(() => {
      setAutostartState(!enabled);
      setFailed(true);
    });
  }, []);

  const recheck = useCallback(() => {
    setUpdateState({ kind: "checking" });
    checkUpdate()
      .then((found) => setUpdateState(toUpdateState(found)))
      .catch(() => setUpdateState({ kind: "error" }));
  }, []);

  const install = useCallback((info: UpdateInfo) => {
    setUpdateState({ kind: "installing", info, progress: null });
    // On success the installer replaces this app and this process exits.
    installUpdate().catch(() => setUpdateState({ kind: "installFailed", info }));
  }, []);

  const showNotes = useCallback((info: UpdateInfo) => {
    if (info.notes) openReleaseNotes(info.notes).catch(() => {});
  }, []);

  const showProject = useCallback(() => {
    openRepository().then(() => setProjectError(false)).catch(() => setProjectError(true));
  }, []);

  // Two clicks: the first arms the button, the second resets. Pinned and
  // recent apps stay; they belong to the user, not to the settings.
  const reset = useCallback(() => {
    if (!confirmingReset) {
      setConfirmingReset(true);
      resetTimer.current = window.setTimeout(() => setConfirmingReset(false), RESET_CONFIRM_MS);
      return;
    }
    setConfirmingReset(false);
    defaultConfig()
      .then(({ pinned: _pinned, recent: _recent, shortcutsFolder: _folder, ...defaults }) =>
        update(defaults),
      )
      .catch(() => setFailed(true));
  }, [confirmingReset, update]);

  if (!config) return <div className="window" />;
  const t = textsFor(config.language);

  const isExplorerPinned = config.pinned.some((p) => p.toLowerCase().endsWith("explorer.exe"));
  const toggleExplorer = useCallback((pinned: boolean) => {
    setExplorerPinned(pinned)
      .then((saved) => setConfig(saved))
      .catch(() => setFailed(true));
  }, []);
  const handleRestoreLayout = useCallback(() => {
    restoreDefaultTaskbarLayout()
      .then((saved) => setConfig(saved))
      .catch(() => setFailed(true));
  }, []);

  const currentDurationMs =
    config.genieDurationMs ??
    (config.genieSpeed === "faster" ? 250 : config.genieSpeed === "fast" ? 350 : 500);

  const sliderVal =
    GENIE_DURATION_MS_RANGE.max + GENIE_DURATION_MS_RANGE.min - currentDurationMs;

  const handleGenieSliderChange = (newSliderVal: number) => {
    const duration = Math.round(
      GENIE_DURATION_MS_RANGE.max + GENIE_DURATION_MS_RANGE.min - newSliderVal,
    );
    const speedEnum: GenieSpeed =
      duration <= 275 ? "faster" : duration <= 400 ? "fast" : "normal";
    update({ genieDurationMs: duration, genieSpeed: speedEnum });
  };

  const resetGenieSpeed = () => {
    update({
      genieDurationMs: DEFAULT_GENIE_DURATION_MS,
      genieSpeed: "normal",
    });
  };

  const chooseFolder = () => {
    pickFolder(config.shortcutsFolder)
      .then((folder) => folder && update({ shortcutsFolder: folder }))
      .catch(() => setFailed(true));
  };

  const handleChangeAppIcon = useCallback((appId: string) => {
    pickIconFile()
      .then((path) => {
        if (path) {
          setCustomIcon(appId, path).then(setConfig).catch(() => setFailed(true));
        }
      })
      .catch(() => setFailed(true));
  }, []);

  const handleResetAppIcon = useCallback((appId: string) => {
    setCustomIcon(appId, null).then(setConfig).catch(() => setFailed(true));
  }, []);

  const handlePickNewCustomIcon = useCallback(() => {
    pickIconFile()
      .then((path) => {
        if (path) {
          const pwa = config.pinned.find((p) => p.startsWith("pwa:")) || config.pinned[0];
          const target = pwa || "custom_app";
          setCustomIcon(target, path).then(setConfig).catch(() => setFailed(true));
        }
      })
      .catch(() => setFailed(true));
  }, [config.pinned]);
  const appWindow = getCurrentWindow();
  const waiting = pendingUpdate(updateState);
  // An update the user asked for keeps its banner even after "Later".
  const showBanner =
    waiting !== null && (!bannerDismissed || updateState.kind !== "available");

  return (
    <div className="window">
      <aside className="sidebar" data-tauri-drag-region>
        <div className="traffic">
          <button
            type="button"
            className="light light-close"
            aria-label={t.close}
            onClick={() => appWindow.close()}
          />
          <button
            type="button"
            className="light light-minimize"
            aria-label={t.minimize}
            onClick={() => appWindow.minimize()}
          />
          <button
            type="button"
            className="light light-zoom"
            aria-label={t.zoom}
            onClick={() => appWindow.toggleMaximize()}
          />
        </div>
        <nav>
          {SECTIONS.map((id) => (
            <NavItem
              key={id}
              section={id}
              active={section === id}
              onSelect={() => setSection(id)}
              badge={id === "system" && waiting ? t.updateBadge : undefined}
            >
              {sectionTitle(t, id)}
            </NavItem>
          ))}
        </nav>
        <footer className="sidebar-footer">
          <button type="button" className="project-link" onClick={showProject}>
            <SettingIcon name="star" size={20} />
            <span><strong>{t.githubTitle}</strong><span>{t.githubStar}</span></span>
            <SettingIcon name="external" size={12} />
          </button>
          {projectError ? <p className="project-error" role="alert">{t.githubError}</p> : null}
          <div className="sidebar-version"><span>Flote</span><span>{version ? `v${version}` : ""}</span></div>
        </footer>
      </aside>

      <main className="content">
        <header className="content-title" data-tauri-drag-region>
          {sectionTitle(t, section)}
        </header>
        {showBanner && waiting ? (
          <UpdateBanner
            t={t}
            info={waiting}
            progress={updateState.kind === "installing" ? updateState.progress : null}
            installing={updateState.kind === "installing"}
            failed={updateState.kind === "installFailed"}
            onInstall={() => install(waiting)}
            onLater={() => setBannerDismissed(true)}
            onNotes={() => showNotes(waiting)}
          />
        ) : null}
        <div className="content-body">
          {section === "dock" ? (
            <>
              <Card title={t.groupAppearance}>
                <VisualChoices<DockPosition>
                  label={t.dockPosition}
                  value={config.dockPosition}
                  options={[
                    { value: "left", label: t.dockPositionLeft, preview: <DockPreview position="left" iconSize={config.iconSize} wallpaper={wallpaper} /> },
                    { value: "bottom", label: t.dockPositionBottom, preview: <DockPreview position="bottom" iconSize={config.iconSize} wallpaper={wallpaper} /> },
                    { value: "right", label: t.dockPositionRight, preview: <DockPreview position="right" iconSize={config.iconSize} wallpaper={wallpaper} /> },
                  ]}
                  onChange={(dockPosition) => update({ dockPosition })}
                />
                <Row label={t.size}>
                  <Slider
                    label={t.size}
                    value={config.iconSize}
                    min={ICON_SIZE.min}
                    max={ICON_SIZE.max}
                    step={1}
                    unit=" px"
                    displayValue={`${config.iconSize} px`}
                    onChange={(iconSize) => update({ iconSize })}
                  />
                </Row>
                <Row icon={<SettingIcon name="magnification" />} label={t.magnification} hint={t.magnificationHint}>
                  <Toggle
                    label={t.magnification}
                    checked={config.magnification}
                    onChange={(magnification) => update({ magnification })}
                  />
                </Row>
                <SubRow label={t.magnifiedSize} open={config.magnification}>
                  <Slider
                    label={t.magnifiedSize}
                    value={config.magnifiedSize}
                    min={config.iconSize}
                    max={MAX_MAGNIFIED_SIZE}
                    onChange={(magnifiedSize) => update({ magnifiedSize })}
                  />
                </SubRow>
              </Card>
              <Card title={t.groupBehavior}>
                <Row icon={<SettingIcon name="hide" />} label={t.autoHide} hint={t.autoHideHint}>
                  <Toggle
                    label={t.autoHide}
                    checked={config.autoHide}
                    onChange={(autoHide) => update({ autoHide })}
                  />
                </Row>
                <Row icon={<SettingIcon name="bounce" />} label={t.bounceOnOpen}>
                  <Toggle
                    label={t.bounceOnOpen}
                    checked={config.bounceOnOpen}
                    onChange={(bounceOnOpen) => update({ bounceOnOpen })}
                  />
                </Row>
                <Row icon={<SettingIcon name="mascot" />} label={t.mascot} hint={t.mascotHint}>
                  <Toggle
                    label={t.mascot}
                    checked={config.mascot}
                    onChange={(mascot) => update({ mascot })}
                  />
                </Row>
                <Row icon={<SettingIcon name="indicators" />} label={t.showIndicators}>
                  <Toggle
                    label={t.showIndicators}
                    checked={config.showIndicators}
                    onChange={(showIndicators) => update({ showIndicators })}
                  />
                </Row>
                <Row icon={<SettingIcon name="recents" />} label={t.showRecents} hint={t.showRecentsHint}>
                  <Toggle
                    label={t.showRecents}
                    checked={config.showRecents}
                    onChange={(showRecents) => update({ showRecents })}
                  />
                </Row>
              </Card>
              <Card title={t.defaultItemsTitle}>
                <Row icon={<SettingIcon name="start" />} label={t.showStart} hint={t.showStartHint}>
                  <Toggle
                    label={t.showStart}
                    checked={config.showStart}
                    onChange={(showStart) => update({ showStart })}
                  />
                </Row>
                <Row icon={<SettingIcon name="windows" />} label={t.showExplorer} hint={t.showExplorerHint}>
                  <Toggle
                    label={t.showExplorer}
                    checked={isExplorerPinned}
                    onChange={(pinned) => toggleExplorer(pinned)}
                  />
                </Row>
                <Row icon={<SettingIcon name="folder" />} label={t.showDownloads} hint={t.showDownloadsHint}>
                  <Toggle
                    label={t.showDownloads}
                    checked={config.showDownloads}
                    onChange={(showDownloads) => update({ showDownloads })}
                  />
                </Row>
                <Row icon={<SettingIcon name="reset" />} label={t.showRecycleBin} hint={t.showRecycleBinHint}>
                  <Toggle
                    label={t.showRecycleBin}
                    checked={config.showRecycleBin}
                    onChange={(showRecycleBin) => update({ showRecycleBin })}
                  />
                </Row>
                <Row label={t.restoreDefaultItems} hint={t.restoreDefaultItemsHint}>
                  <button type="button" className="push" onClick={handleRestoreLayout}>
                    {t.restoreDefaultItemsButton}
                  </button>
                </Row>
              </Card>
              <Card>
                <Row icon={<SettingIcon name="folder" />} label={t.shortcutsFolder} hint={config.shortcutsFolder ?? t.shortcutsFolderHint}>
                  <div className="push-group">
                    {config.shortcutsFolder ? (
                      <button type="button" className="push" onClick={() => update({ shortcutsFolder: null })}>
                        {t.shortcutsFolderRemove}
                      </button>
                    ) : null}
                    <button type="button" className="push" onClick={chooseFolder}>
                      {t.shortcutsFolderChoose}
                    </button>
                  </div>
                </Row>
              </Card>
              <Card title={t.webAppsTitle}>
                <Row label={t.webAppsTitle} hint={t.webAppsHint} stacked>
                  <div className="custom-icons-list">
                    {Object.entries(config.customIcons ?? {}).map(([appId, iconPath]) => (
                      <div key={appId} className="custom-icon-row">
                        <span className="custom-icon-name" title={appId}>
                          {appId.startsWith("pwa:") ? `PWA: ${appId.replace("pwa:", "")}` : appId.split(/[\\/]/).pop()}
                        </span>
                        <span className="custom-icon-path" title={iconPath}>
                          {iconPath.split(/[\\/]/).pop()}
                        </span>
                        <div className="push-group">
                          <button
                            type="button"
                            className="push"
                            onClick={() => handleChangeAppIcon(appId)}
                          >
                            {t.changeIcon}
                          </button>
                          <button
                            type="button"
                            className="push"
                            onClick={() => handleResetAppIcon(appId)}
                          >
                            {t.resetIcon}
                          </button>
                        </div>
                      </div>
                    ))}
                    {Object.keys(config.customIcons ?? {}).length === 0 ? (
                      <p className="custom-icons-empty">{t.noCustomIcons}</p>
                    ) : null}
                    <div className="push-group" style={{ marginTop: "8px" }}>
                      <button
                        type="button"
                        className="push"
                        onClick={handlePickNewCustomIcon}
                      >
                        {t.addCustomIcon}
                      </button>
                    </div>
                  </div>
                </Row>
              </Card>
            </>
          ) : section === "windows" ? (
            <>
              <Card title={t.groupMinimize}>
                <Row icon={<SettingIcon name="genie" />} label={t.genie} hint={t.genieHint}>
                  <Toggle
                    label={t.genie}
                    checked={config.genie}
                    onChange={(genie) => update({ genie })}
                  />
                </Row>
                <SubRow label={t.genieSpeed} hint={t.genieSpeedHint} open={config.genie} stacked>
                  <GenieSpeedPreview durationMs={currentDurationMs} />
                  <div className="genie-speed-controls">
                    <div className="genie-slider-row">
                      <span className="genie-speed-label">{t.genieSpeedSlow}</span>
                      <Slider
                        label={t.genieSpeed}
                        min={GENIE_DURATION_MS_RANGE.min}
                        max={GENIE_DURATION_MS_RANGE.max}
                        step={25}
                        value={sliderVal}
                        displayValue={`${(currentDurationMs / 1000).toFixed(2)} s`}
                        onChange={handleGenieSliderChange}
                      />
                      <span className="genie-speed-label">{t.genieSpeedFast}</span>
                    </div>
                    <div className="genie-speed-footer">
                      <button
                        type="button"
                        className="push push-sm"
                        onClick={resetGenieSpeed}
                      >
                        {t.genieSpeedReset}
                      </button>
                    </div>
                  </div>
                </SubRow>
                <SubRow label={t.genieOff} hint={t.genieOffHint} open={config.genie && !animations}>
                  <button type="button" className="push" onClick={() => void openAnimationSettings()}>
                    {t.genieOffAction}
                  </button>
                </SubRow>
                <Row icon={<SettingIcon name="windows" />} label={t.showMinimized} hint={t.showMinimizedHint}>
                  <Toggle
                    label={t.showMinimized}
                    checked={config.showMinimized}
                    onChange={(showMinimized) => update({ showMinimized })}
                  />
                </Row>
              </Card>
            </>
          ) : section === "menubar" ? (
            <>
              <Card>
                <Row icon={<SettingIcon name="menu" />} label={t.menuBarShow} hint={t.menuBarHint}>
                  <Toggle
                    label={t.menuBarShow}
                    checked={config.menuBar}
                    onChange={(menuBar) => update({ menuBar })}
                  />
                </Row>
                <SubRow label={t.menuBarAutoHide} hint={t.menuBarAutoHideHint} open={config.menuBar}>
                  <Toggle
                    label={t.menuBarAutoHide}
                    checked={config.menuBarAutoHide}
                    onChange={(menuBarAutoHide) => update({ menuBarAutoHide })}
                  />
                </SubRow>
                <SubRow label={t.menuBarMusic} hint={t.menuBarMusicHint} open={config.menuBar}>
                  <Toggle
                    label={t.menuBarMusic}
                    checked={config.menuBarMusic}
                    onChange={(menuBarMusic) => update({ menuBarMusic })}
                  />
                </SubRow>
                <SubRow label={t.aiUsage} hint={t.aiUsageHint} open={config.menuBar}>
                  <Toggle
                    label={t.aiUsage}
                    checked={config.aiUsage}
                    onChange={(aiUsage) => update({ aiUsage })}
                  />
                </SubRow>
                <SubRow
                  label={t.islandStyle}
                  stacked
                  open={config.menuBar && (config.menuBarMusic || config.aiUsage)}
                >
                  <VisualChoices<IslandStyle>
                    label={t.islandStyle}
                    hideLabel
                    value={config.menuBarIslandStyle}
                    options={[
                      { value: "notch", label: t.islandStyleNotch, preview: <IslandPreview style="notch" wallpaper={wallpaper} /> },
                      { value: "bubble", label: t.islandStyleBubble, preview: <IslandPreview style="bubble" wallpaper={wallpaper} /> },
                    ]}
                    onChange={(menuBarIslandStyle) => update({ menuBarIslandStyle })}
                  />
                </SubRow>
                <SubRow label={t.accessoryBattery} hint={t.accessoryBatteryHint} open={config.menuBar}>
                  <Toggle
                    label={t.accessoryBattery}
                    checked={config.accessoryBattery}
                    onChange={(accessoryBattery) => update({ accessoryBattery })}
                  />
                </SubRow>
              </Card>
              {config.menuBar && (
                <Card title={t.menuBarItems}>
                  <Row icon={<SettingIcon name="bluetooth" />} label={t.menuBarBluetooth} hint={t.menuBarBluetoothHint}>
                    <Toggle
                      label={t.menuBarBluetooth}
                      checked={config.menuBarBluetooth}
                      onChange={(menuBarBluetooth) => update({ menuBarBluetooth })}
                    />
                  </Row>
                  <Row icon={<SettingIcon name="battery" />} label={t.menuBarBatteryPercent} hint={t.menuBarBatteryPercentHint}>
                    <Toggle
                      label={t.menuBarBatteryPercent}
                      checked={config.menuBarBatteryPercent ?? false}
                      onChange={(menuBarBatteryPercent) => update({ menuBarBatteryPercent })}
                    />
                  </Row>
                  <Row icon={<SettingIcon name="search" />} label={t.menuBarSearch} hint={t.menuBarSearchHint}>
                    <Toggle
                      label={t.menuBarSearch}
                      checked={config.menuBarSearch ?? true}
                      onChange={(menuBarSearch) => update({ menuBarSearch })}
                    />
                  </Row>
                  <Row icon={<SettingIcon name="windows" />} label={t.menuBarTaskView} hint={t.menuBarTaskViewHint}>
                    <Toggle
                      label={t.menuBarTaskView}
                      checked={config.menuBarTaskView ?? true}
                      onChange={(menuBarTaskView) => update({ menuBarTaskView })}
                    />
                  </Row>
                  <Row icon={<SettingIcon name="widgets" />} label={t.menuBarWidgets} hint={t.menuBarWidgetsHint}>
                    <Toggle
                      label={t.menuBarWidgets}
                      checked={config.menuBarWidgets ?? true}
                      onChange={(menuBarWidgets) => update({ menuBarWidgets })}
                    />
                  </Row>
                </Card>
              )}
            </>
          ) : (
            <>
              <Card>
                <Row icon={<SettingIcon name="start" />} label={t.autostart}>
                  <Toggle label={t.autostart} checked={autostart} onChange={changeAutostart} />
                </Row>
                <Row icon={<SettingIcon name="language" />} label={t.language}>
                  <Segmented<Language>
                    label={t.language}
                    value={config.language}
                    options={[
                      { value: "auto", label: t.languageAuto },
                      { value: "es", label: "Español" },
                      { value: "en", label: "English" },
                      { value: "fr", label: "Français" },
                    ]}
                    onChange={(language) => update({ language })}
                  />
                </Row>
              </Card>
              <Card title={t.updates}>
                <Row icon={<SettingIcon name="updates" />} label={t.checkUpdates} hint={t.checkUpdatesHint}>
                  <Toggle
                    label={t.checkUpdates}
                    checked={config.checkUpdates}
                    onChange={(checkUpdates) => update({ checkUpdates })}
                  />
                </Row>
                <SubRow label={t.autoUpdate} hint={t.autoUpdateHint} open={config.checkUpdates}>
                  <Toggle
                    label={t.autoUpdate}
                    checked={config.autoUpdate}
                    onChange={(autoUpdate) => update({ autoUpdate })}
                  />
                </SubRow>
                <Row icon={<SettingIcon name="updates" />} label={t.updatesNow} hint={updateHint(t, updateState, version)}>
                  {waiting && updateState.kind !== "installing" ? (
                    <button type="button" className="push push-primary" onClick={() => install(waiting)}>
                      {t.updatesInstall}
                    </button>
                  ) : (
                    <button
                      type="button"
                      className="push"
                      disabled={updateState.kind === "checking" || updateState.kind === "installing"}
                      onClick={recheck}
                    >
                      {t.updatesCheck}
                    </button>
                  )}
                </Row>
              </Card>
              <Card>
                <Row icon={<SettingIcon name="reset" />} label={t.reset} hint={confirmingReset ? t.resetConfirmHint : t.resetHint}>
                  <button
                    type="button"
                    className={confirmingReset ? "push push-danger" : "push"}
                    onClick={reset}
                  >
                    {confirmingReset ? t.resetConfirm : t.resetButton}
                  </button>
                </Row>
              </Card>
            </>
          )}
          {failed ? (
            <p className="error" role="alert">
              {t.saveError}
            </p>
          ) : null}
        </div>
      </main>
    </div>
  );
}

function updateHint(t: Texts, state: UpdateState, version: string): string {
  switch (state.kind) {
    case "off":
      return t.updatesOff;
    case "checking":
      return t.updatesChecking;
    case "current":
      return t.updatesCurrent.replace("{version}", version);
    case "available":
      return t.updatesAvailable.replace("{version}", state.info.version);
    case "installing":
      return t.updatesInstalling;
    case "installFailed":
      return t.updateInstallError;
    case "error":
      return t.updatesError;
  }
}

const SECTIONS: readonly SectionId[] = ["dock", "windows", "menubar", "system"];

function sectionTitle(t: Texts, section: SectionId): string {
  switch (section) {
    case "dock":
      return t.dock;
    case "windows":
      return t.windows;
    case "menubar":
      return t.menuBar;
    case "system":
      return t.system;
  }
}

interface NavItemProps {
  section: SectionId;
  active: boolean;
  onSelect: () => void;
  /** Accessible text of a red badge, as macOS shows for a pending update. */
  badge?: string;
  children: string;
}

function NavItem({ section, active, onSelect, badge, children }: NavItemProps) {
  return (
    <button
      type="button"
      className="nav-item"
      aria-current={active ? "page" : undefined}
      onClick={onSelect}
    >
      <SectionIcon section={section} />
      {children}
      {badge ? (
        <span className="nav-badge" role="img" aria-label={badge}>
          1
        </span>
      ) : null}
    </button>
  );
}
