import {
  Check,
  Crosshair,
  Download,
  FileJson,
  FileUp,
  Eye,
  EyeOff,
  Keyboard,
  Layers3,
  Maximize2,
  Move,
  Palette,
  PanelLeft,
  RadioTower,
  Search,
  Settings,
  ShieldAlert,
  ShieldCheck,
  Upload,
} from "lucide-react";
import type { CSSProperties } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import "./App.css";
import type {
  EffectiveKey,
  ImportCandidate,
  KeyPeekDiscoveredDevice,
  KeyboardSnapshot,
  RuntimeEvent,
  SourceConflict,
  SourceRef,
  StyleDensity,
  VisibilityPolicy,
} from "./domain";
import { navLayerEvent } from "./fixtures";
import { applyRuntimeEvent, promoteImportCandidateSource } from "./state";
import {
  commitImportCandidate,
  discoverKeyPeekDevices,
  importKeyPeekQmkInfoFile,
  importKeyvizStyleFile,
  importOverkeysCompanionFile,
  importVialDevice,
  importVialFile,
  importZmkKeymapFile,
  loadLaunchAtLogin,
  loadFakeRuntimeEvents,
  loadInitialSnapshot,
  listenToRuntimeEvents,
  loadActiveProfileEdn,
  promoteActiveSourceCandidate,
  refreshHostPermissionHealth,
  registerSentinelKeyShortcuts,
  requestHostInputPermissions,
  saveActiveProfileEdn,
  setLaunchAtLogin,
  startKanataTcpBackend,
  startOverlayDrag,
  startOverlayResize,
  startKeyPeekLiveBackend,
  setOverlayPositioningMode,
  setOverlayVisibilityPolicy,
  setOverlayVisible,
  setVisualStyleDensity,
  stopKanataTcpBackend,
  unregisterSentinelKeyShortcuts,
} from "./tauriClient";

type View = "overlay" | "inspector" | "import" | "settings";

export const FADE_VISIBILITY_INACTIVITY_MS = 3_000;

function snapshotWithOverlayVisible(snapshot: KeyboardSnapshot, visible: boolean): KeyboardSnapshot {
  return {
    ...snapshot,
    overlay_window: {
      ...snapshot.overlay_window,
      visible,
      positioning_mode: visible ? snapshot.overlay_window.positioning_mode : false,
      click_through: visible ? snapshot.overlay_window.click_through : true,
    },
  };
}

function mergeOverlayWindowSnapshot(
  current: KeyboardSnapshot,
  overlaySnapshot: KeyboardSnapshot,
): KeyboardSnapshot {
  return {
    ...current,
    overlay_window: overlaySnapshot.overlay_window,
    runtime_state: {
      ...current.runtime_state,
      backend_health: overlaySnapshot.runtime_state.backend_health,
    },
    backends: overlaySnapshot.backends,
  };
}

function App() {
  const overlayOnly = window.location.hash === "#/overlay";
  const [view, setView] = useState<View>(overlayOnly ? "overlay" : "overlay");
  const [snapshot, setSnapshot] = useState<KeyboardSnapshot | null>(null);
  const snapshotRef = useRef<KeyboardSnapshot | null>(null);
  const fadeTimerRef = useRef<ReturnType<typeof window.setTimeout> | null>(null);
  const [events, setEvents] = useState<RuntimeEvent[]>([]);
  const [eventIndex, setEventIndex] = useState(0);
  const [importCandidate, setImportCandidate] = useState<ImportCandidate | null>(null);
  const [importError, setImportError] = useState<string | null>(null);
  const [profileStatus, setProfileStatus] = useState<string | null>(null);
  const [keyPeekVid, setKeyPeekVid] = useState("");
  const [keyPeekPid, setKeyPeekPid] = useState("");
  const [keyPeekDevices, setKeyPeekDevices] = useState<KeyPeekDiscoveredDevice[]>([]);
  const [kanataHost, setKanataHost] = useState("127.0.0.1");
  const [kanataPort, setKanataPort] = useState("7070");
  const [launchAtLogin, setLaunchAtLoginState] = useState<boolean | null>(null);

  useEffect(() => {
    void loadInitialSnapshot().then((initialSnapshot) => {
      setSnapshot(initialSnapshot);
      void refreshHostPermissionHealth().then((permissionSnapshot) => {
        if (permissionSnapshot) {
          setSnapshot(permissionSnapshot);
        }
      });
    });
    void loadFakeRuntimeEvents().then(setEvents);
    void loadLaunchAtLogin().then(setLaunchAtLoginState);
  }, []);

  useEffect(() => {
    snapshotRef.current = snapshot;
  }, [snapshot]);

  const kanataProfileConfig = snapshot ? kanataTcpSettingsFromSnapshot(snapshot) : null;

  useEffect(() => {
    if (!kanataProfileConfig) return;
    setKanataHost(kanataProfileConfig.host);
    setKanataPort(kanataProfileConfig.port);
  }, [kanataProfileConfig?.host, kanataProfileConfig?.port, snapshot?.profile_id]);

  const clearFadeTimer = useCallback(() => {
    if (fadeTimerRef.current !== null) {
      window.clearTimeout(fadeTimerRef.current);
      fadeTimerRef.current = null;
    }
  }, []);

  const applyLocalOverlayVisible = useCallback((visible: boolean) => {
    setSnapshot((current) => (current ? snapshotWithOverlayVisible(current, visible) : current));
  }, []);

  const syncOverlayWindowVisible = useCallback(
    async (visible: boolean) => {
      applyLocalOverlayVisible(visible);
      const nextSnapshot = await setOverlayVisible(visible);
      if (nextSnapshot) {
        setSnapshot((current) =>
          current ? mergeOverlayWindowSnapshot(current, nextSnapshot) : nextSnapshot,
        );
      }
      return nextSnapshot;
    },
    [applyLocalOverlayVisible],
  );

  const scheduleFadeHide = useCallback(() => {
    clearFadeTimer();
    fadeTimerRef.current = window.setTimeout(() => {
      fadeTimerRef.current = null;
      const current = snapshotRef.current;
      if (
        current?.overlay_window.visibility === "fade" &&
        current.overlay_window.visible &&
        !current.overlay_window.positioning_mode
      ) {
        void syncOverlayWindowVisible(false);
      }
    }, FADE_VISIBILITY_INACTIVITY_MS);
  }, [clearFadeTimer, syncOverlayWindowVisible]);

  const noteRuntimeActivity = useCallback(() => {
    const current = snapshotRef.current;
    if (current?.overlay_window.visibility !== "fade") return;
    if (!current.overlay_window.visible) {
      void syncOverlayWindowVisible(true);
    }
    scheduleFadeHide();
  }, [scheduleFadeHide, syncOverlayWindowVisible]);

  const handleRuntimeEvent = useCallback(
    (event: RuntimeEvent) => {
      setSnapshot((current) => (current ? applyRuntimeEvent(current, event) : current));
      noteRuntimeActivity();
    },
    [noteRuntimeActivity],
  );

  useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | null = null;

    void listenToRuntimeEvents(handleRuntimeEvent).then((nextUnlisten) => {
      if (!mounted) {
        nextUnlisten?.();
        return;
      }
      unlisten = nextUnlisten;
    });

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, [handleRuntimeEvent]);

  useEffect(() => {
    if (
      snapshot?.overlay_window.visibility === "fade" &&
      snapshot.overlay_window.visible &&
      !snapshot.overlay_window.positioning_mode
    ) {
      scheduleFadeHide();
      return;
    }
    clearFadeTimer();
  }, [
    clearFadeTimer,
    scheduleFadeHide,
    snapshot?.overlay_window.positioning_mode,
    snapshot?.overlay_window.visibility,
    snapshot?.overlay_window.visible,
  ]);

  useEffect(() => () => clearFadeTimer(), [clearFadeTimer]);

  const activeLayer = snapshot?.runtime_state.layer_stack[0];
  const health = snapshot?.runtime_state.backend_health ?? [];
  const topHealth = health[0];
  const kanataHealth = health.find((candidate) => candidate.backend_id === "kanata-tcp");
  const sentinelHealth = health.find((candidate) => candidate.backend_id === "sentinel-keys");
  const sentinelKeysEnabled = sentinelHealth ? sentinelHealth.state === "ok" : null;

  async function togglePositioningMode() {
    if (!snapshot) return;
    const enabled = !snapshot.overlay_window.positioning_mode;
    const nextSnapshot = await setOverlayPositioningMode(enabled);
    if (nextSnapshot) {
      setSnapshot(nextSnapshot);
      return;
    }
    const fallbackSnapshot: KeyboardSnapshot = {
      ...snapshot,
      overlay_window: {
        ...snapshot.overlay_window,
        positioning_mode: enabled,
        click_through: !enabled,
      },
    };
    setSnapshot(fallbackSnapshot);
  }

  async function updateVisualStyleDensity(density: StyleDensity) {
    setProfileStatus(null);
    const nextSnapshot = await setVisualStyleDensity(density);
    if (nextSnapshot) {
      setSnapshot(nextSnapshot);
      setProfileStatus(`Visual style density set to ${density}`);
      return;
    }

    setSnapshot((current) =>
      current
        ? {
            ...current,
            visual_style: {
              ...current.visual_style,
              density,
            },
          }
        : current,
    );
    setProfileStatus(`Visual style density set to ${density}`);
  }

  async function updateOverlayVisibilityPolicy(visibility: VisibilityPolicy) {
    setProfileStatus(null);
    const nextSnapshot = await setOverlayVisibilityPolicy(visibility);
    if (nextSnapshot) {
      setSnapshot(nextSnapshot);
      setProfileStatus(`Overlay Visibility Policy set to ${visibility}`);
      return;
    }

    setSnapshot((current) =>
      current
        ? {
            ...current,
            overlay_window: {
              ...current.overlay_window,
              visibility,
              visible:
                visibility === "pinned" || visibility === "fade"
                  ? true
                  : current.overlay_window.visible,
            },
          }
        : current,
    );
    setProfileStatus(`Overlay Visibility Policy set to ${visibility}`);
  }

  async function updateOverlayVisible(visible: boolean) {
    setProfileStatus(null);
    await syncOverlayWindowVisible(visible);
    setProfileStatus(visible ? "Overlay Window shown" : "Overlay Window hidden");
  }

  function advanceFakeEvent() {
    if (!snapshotRef.current) return;
    const event = events[eventIndex] ?? navLayerEvent;
    setSnapshot((current) => (current ? applyRuntimeEvent(current, event) : current));
    noteRuntimeActivity();
    setEventIndex((index) => (index + 1) % Math.max(events.length, 1));
  }

  async function promoteSourceValue(conflict: SourceConflict, sourceId: string) {
    if (!snapshot) return;
    setSnapshot(await promoteActiveSourceCandidate(snapshot, conflict, sourceId));
  }

  function promoteImportValue(conflict: SourceConflict, sourceId: string) {
    setImportCandidate((candidate) =>
      candidate ? promoteImportCandidateSource(candidate, conflict, sourceId) : candidate,
    );
  }

  async function dragOverlay() {
    const nextSnapshot = await startOverlayDrag();
    if (nextSnapshot) {
      setSnapshot(nextSnapshot);
    }
  }

  async function resizeOverlay() {
    const nextSnapshot = await startOverlayResize("south-east");
    if (nextSnapshot) {
      setSnapshot(nextSnapshot);
    }
  }

  async function connectKeyPeekLive() {
    setProfileStatus(null);
    const nextSnapshot = await startKeyPeekLiveBackend({ vid: keyPeekVid, pid: keyPeekPid });
    if (!nextSnapshot) {
      setProfileStatus("KeyPeek live connection unavailable");
      return;
    }
    setSnapshot(nextSnapshot);
    const health = nextSnapshot.runtime_state.backend_health.find(
      (candidate) => candidate.backend_id === "keypeek-live",
    );
    setProfileStatus(health?.message ?? "KeyPeek live backend updated");
  }

  async function discoverKeyPeekLiveDevices() {
    setProfileStatus(null);
    const discovery = await discoverKeyPeekDevices();
    if (!discovery) {
      setProfileStatus("KeyPeek device discovery unavailable");
      return;
    }

    setKeyPeekDevices(discovery.devices);
    setSnapshot(discovery.snapshot);
    const firstDevice = discovery.devices[0];
    if (firstDevice) {
      setKeyPeekVid(firstDevice.vid);
      setKeyPeekPid(firstDevice.pid);
    }
    const health = discovery.snapshot.runtime_state.backend_health.find(
      (candidate) => candidate.backend_id === "keypeek-live",
    );
    setProfileStatus(health?.message ?? "KeyPeek device discovery complete");
  }

  function selectKeyPeekDevice(index: string) {
    const device = keyPeekDevices[Number(index)];
    if (!device) return;
    setKeyPeekVid(device.vid);
    setKeyPeekPid(device.pid);
  }

  async function importVialDevicePreview() {
    setImportError(null);
    setProfileStatus(null);
    try {
      setImportCandidate(await importVialDevice({ vid: keyPeekVid, pid: keyPeekPid }));
      setView("import");
    } catch (error) {
      setImportError(error instanceof Error ? error.message : String(error));
      setView("import");
    }
  }

  async function updateLaunchAtLogin(enabled: boolean) {
    setProfileStatus(null);
    const next = await setLaunchAtLogin(enabled);
    setLaunchAtLoginState(next);
    setProfileStatus(
      next === null
        ? "Launch at login unavailable"
        : next
          ? "Launch at login enabled"
          : "Launch at login disabled",
    );
  }

  async function updateSentinelKeyShortcuts(enabled: boolean) {
    setProfileStatus(null);
    const nextSnapshot = enabled
      ? await registerSentinelKeyShortcuts()
      : await unregisterSentinelKeyShortcuts();
    if (!nextSnapshot) {
      setProfileStatus("Sentinel Key shortcuts unavailable");
      return;
    }

    setSnapshot(nextSnapshot);
    const health = nextSnapshot.runtime_state.backend_health.find(
      (candidate) => candidate.backend_id === "sentinel-keys",
    );
    setProfileStatus(health?.message ?? "Sentinel Keys updated");
  }

  async function updateHostPermissionHealth(requestPermissions: boolean) {
    setProfileStatus(null);
    const nextSnapshot = requestPermissions
      ? await requestHostInputPermissions()
      : await refreshHostPermissionHealth();
    if (!nextSnapshot) {
      setProfileStatus("Host Input permission health unavailable");
      return;
    }

    setSnapshot(nextSnapshot);
    const health = nextSnapshot.runtime_state.backend_health.find(
      (candidate) => candidate.backend_id === "sentinel-keys",
    );
    setProfileStatus(health?.message ?? "Host Input permission health updated");
  }

  async function connectKanataTcp() {
    setProfileStatus(null);
    const parsedPort = Number.parseInt(kanataPort, 10);
    if (!Number.isInteger(parsedPort) || parsedPort < 1 || parsedPort > 65535) {
      setProfileStatus("Kanata TCP port must be between 1 and 65535");
      return;
    }
    const nextSnapshot = await startKanataTcpBackend({ host: kanataHost, port: parsedPort });
    if (!nextSnapshot) {
      setProfileStatus("Kanata TCP connection unavailable");
      return;
    }

    setSnapshot(nextSnapshot);
    const health = nextSnapshot.runtime_state.backend_health.find(
      (candidate) => candidate.backend_id === "kanata-tcp",
    );
    setProfileStatus(health?.message ?? "Kanata TCP backend updated");
  }

  async function disconnectKanataTcp() {
    setProfileStatus(null);
    const nextSnapshot = await stopKanataTcpBackend();
    if (!nextSnapshot) {
      setProfileStatus("Kanata TCP backend unavailable");
      return;
    }

    setSnapshot(nextSnapshot);
    const health = nextSnapshot.runtime_state.backend_health.find(
      (candidate) => candidate.backend_id === "kanata-tcp",
    );
    setProfileStatus(health?.message ?? "Kanata TCP backend stopped");
  }

  async function handleImport(file: File | null) {
    if (!file) return;
    setImportError(null);
    setProfileStatus(null);
    try {
      const contents = await file.text();
      setImportCandidate(await importVialFile(contents));
      setView("import");
    } catch (error) {
      setImportError(error instanceof Error ? error.message : String(error));
    }
  }

  async function handleStyleImport(file: File | null) {
    if (!file) return;
    setImportError(null);
    setProfileStatus(null);
    try {
      const contents = await file.text();
      setImportCandidate(await importKeyvizStyleFile(contents));
      setView("import");
    } catch (error) {
      setImportError(error instanceof Error ? error.message : String(error));
    }
  }

  async function handleKeyPeekQmkImport(file: File | null) {
    if (!file) return;
    setImportError(null);
    setProfileStatus(null);
    try {
      const contents = await file.text();
      setImportCandidate(await importKeyPeekQmkInfoFile(contents));
      setView("import");
    } catch (error) {
      setImportError(error instanceof Error ? error.message : String(error));
    }
  }

  async function handleOverkeysImport(file: File | null) {
    if (!file) return;
    setImportError(null);
    setProfileStatus(null);
    try {
      const contents = await file.text();
      setImportCandidate(await importOverkeysCompanionFile(contents));
      setView("import");
    } catch (error) {
      setImportError(error instanceof Error ? error.message : String(error));
    }
  }

  async function handleZmkImport(file: File | null) {
    if (!file) return;
    setImportError(null);
    setProfileStatus(null);
    try {
      const contents = await file.text();
      setImportCandidate(await importZmkKeymapFile(contents));
      setView("import");
    } catch (error) {
      setImportError(error instanceof Error ? error.message : String(error));
    }
  }

  async function exportActiveProfile() {
    if (!snapshot) return;
    setProfileStatus(null);
    try {
      const contents = await saveActiveProfileEdn();
      downloadTextFile(`${snapshot.profile_id}.keyplane.edn`, contents);
      setProfileStatus("Exported active profile EDN");
    } catch (error) {
      setProfileStatus(error instanceof Error ? error.message : String(error));
    }
  }

  async function handleProfileLoad(file: File | null) {
    if (!file) return;
    setProfileStatus(null);
    try {
      const contents = await file.text();
      const next = await loadActiveProfileEdn(contents);
      setSnapshot(next);
      setImportCandidate(null);
      setView("overlay");
      setProfileStatus(`Loaded ${next.profile_name}`);
    } catch (error) {
      setProfileStatus(error instanceof Error ? error.message : String(error));
    }
  }

  async function commitImportPreview(candidate: ImportCandidate) {
    setImportError(null);
    setProfileStatus(null);
    try {
      setSnapshot(await commitImportCandidate(candidate));
      setImportCandidate(null);
      setView("overlay");
    } catch (error) {
      setImportError(error instanceof Error ? error.message : String(error));
    }
  }

  if (!snapshot) {
    return <main className="loading">Loading Keyplane</main>;
  }

  if (overlayOnly) {
    return (
      <OverlaySurface
        snapshot={snapshot}
        onAdvance={advanceFakeEvent}
        onDragOverlay={dragOverlay}
        onResizeOverlay={resizeOverlay}
        onTogglePositioningMode={togglePositioningMode}
      />
    );
  }

  return (
    <main className="app-shell">
      <aside className="sidebar" aria-label="Keyplane navigation">
        <div className="brand">
          <div className="brand-mark">K</div>
          <div>
            <strong>Keyplane</strong>
            <span>{snapshot.profile_name}</span>
          </div>
        </div>
        <nav className="nav-tabs">
          <button className={view === "overlay" ? "active" : ""} onClick={() => setView("overlay")}>
            <Layers3 size={17} />
            Overlay
          </button>
          <button
            className={view === "inspector" ? "active" : ""}
            onClick={() => setView("inspector")}
          >
            <PanelLeft size={17} />
            Source Inspector
          </button>
          <button className={view === "import" ? "active" : ""} onClick={() => setView("import")}>
            <Upload size={17} />
            Import Review
          </button>
          <button className={view === "settings" ? "active" : ""} onClick={() => setView("settings")}>
            <Settings size={17} />
            Settings
          </button>
        </nav>
        <section className="health-block" aria-label="Backend health">
          {topHealth?.state === "ok" ? <ShieldCheck size={18} /> : <ShieldAlert size={18} />}
          <div>
            <span>{topHealth?.state ?? "unknown"}</span>
            <p>{topHealth?.message ?? "No backend health reported"}</p>
          </div>
        </section>
      </aside>

      <section className="workspace">
        <header className="toolbar">
          <div>
            <span className="eyebrow">Active layer</span>
            <h1>{activeLayer?.layer_id ?? "none"}</h1>
            {profileStatus ? <p className="toolbar-status">{profileStatus}</p> : null}
          </div>
          <div className="toolbar-actions">
            <form
              className="keypeek-connect"
              onSubmit={(event) => {
                event.preventDefault();
                void connectKeyPeekLive();
              }}
            >
              <input
                aria-label="KeyPeek VID"
                placeholder="VID"
                value={keyPeekVid}
                onChange={(event) => setKeyPeekVid(event.currentTarget.value)}
              />
              <input
                aria-label="KeyPeek PID"
                placeholder="PID"
                value={keyPeekPid}
                onChange={(event) => setKeyPeekPid(event.currentTarget.value)}
              />
              <button type="submit">
                <RadioTower size={17} />
                Connect
              </button>
              <button
                type="button"
                aria-label="Scan KeyPeek devices"
                onClick={() => void discoverKeyPeekLiveDevices()}
              >
                <Search size={17} />
                Scan
              </button>
              {keyPeekDevices.length > 0 ? (
                <select
                  aria-label="Discovered KeyPeek device"
                  value={selectedKeyPeekDeviceIndex(keyPeekDevices, keyPeekVid, keyPeekPid)}
                  onChange={(event) => selectKeyPeekDevice(event.currentTarget.value)}
                >
                  <option value="">Select device</option>
                  {keyPeekDevices.map((device, index) => (
                    <option
                      key={`${device.vid}-${device.pid}-${device.serial_number ?? index}`}
                      value={String(index)}
                    >
                      {device.label}
                    </option>
                  ))}
                </select>
              ) : null}
              <button
                type="button"
                aria-label="Import Vial device"
                onClick={() => void importVialDevicePreview()}
              >
                <FileJson size={17} />
                Vial device
              </button>
            </form>
            <button onClick={advanceFakeEvent}>
              <RadioTower size={17} />
              Fake event
            </button>
            <button onClick={togglePositioningMode}>
              {snapshot.overlay_window.positioning_mode ? <Crosshair size={17} /> : <Move size={17} />}
              {snapshot.overlay_window.positioning_mode ? "Done" : "Position"}
            </button>
            <button onClick={() => void exportActiveProfile()}>
              <Download size={17} />
              Save EDN
            </button>
            <label className="file-button">
              <FileUp size={17} />
              Load EDN
              <input
                type="file"
                accept=".edn,text/plain"
                onChange={(event) => void handleProfileLoad(event.currentTarget.files?.[0] ?? null)}
              />
            </label>
            <label className="file-button">
              <FileJson size={17} />
              Vial file
              <input
                type="file"
                accept=".vil,application/json"
                onChange={(event) => void handleImport(event.currentTarget.files?.[0] ?? null)}
              />
            </label>
            <label className="file-button">
              <Palette size={17} />
              Style JSON
              <input
                type="file"
                accept=".json,application/json"
                onChange={(event) => void handleStyleImport(event.currentTarget.files?.[0] ?? null)}
              />
            </label>
            <label className="file-button">
              <FileJson size={17} />
              KeyPeek QMK info
              <input
                type="file"
                accept=".json,application/json"
                onChange={(event) =>
                  void handleKeyPeekQmkImport(event.currentTarget.files?.[0] ?? null)
                }
              />
            </label>
            <label className="file-button">
              <FileJson size={17} />
              OverKeys JSON
              <input
                type="file"
                accept=".json,application/json"
                onChange={(event) => void handleOverkeysImport(event.currentTarget.files?.[0] ?? null)}
              />
            </label>
            <label className="file-button">
              <FileUp size={17} />
              ZMK keymap
              <input
                type="file"
                accept=".keymap,text/plain"
                onChange={(event) => void handleZmkImport(event.currentTarget.files?.[0] ?? null)}
              />
            </label>
          </div>
        </header>

        {view === "overlay" ? (
          <OverlaySurface
            snapshot={snapshot}
            onAdvance={advanceFakeEvent}
            onDragOverlay={dragOverlay}
            onResizeOverlay={resizeOverlay}
            onTogglePositioningMode={togglePositioningMode}
          />
        ) : null}
        {view === "inspector" ? (
          <SourceInspector snapshot={snapshot} onPromote={promoteSourceValue} />
        ) : null}
        {view === "import" ? (
          <ImportReview
            activeSnapshot={snapshot}
            candidate={importCandidate}
            error={importError}
            onCommit={commitImportPreview}
            onPromote={promoteImportValue}
          />
        ) : null}
        {view === "settings" ? (
          <SettingsView
            density={snapshot.visual_style.density}
            hostPermissionMessage={sentinelHealth?.message ?? null}
            hostPermissionState={sentinelHealth?.state ?? null}
            launchAtLogin={launchAtLogin}
            kanataHealthState={kanataHealth?.state ?? null}
            kanataHost={kanataHost}
            kanataPort={kanataPort}
            overlayVisible={snapshot.overlay_window.visible}
            overlayVisibility={snapshot.overlay_window.visibility}
            sentinelKeysEnabled={sentinelKeysEnabled}
            onDensityChange={updateVisualStyleDensity}
            onHostPermissionRefresh={() => updateHostPermissionHealth(false)}
            onHostPermissionRequest={() => updateHostPermissionHealth(true)}
            onLaunchAtLoginChange={updateLaunchAtLogin}
            onKanataConnect={connectKanataTcp}
            onKanataDisconnect={disconnectKanataTcp}
            onKanataHostChange={setKanataHost}
            onKanataPortChange={setKanataPort}
            onOverlayVisibilityChange={updateOverlayVisibilityPolicy}
            onOverlayVisibleChange={updateOverlayVisible}
            onSentinelKeysChange={updateSentinelKeyShortcuts}
          />
        ) : null}
      </section>
    </main>
  );
}

export function OverlaySurface({
  snapshot,
  onAdvance,
  onDragOverlay,
  onResizeOverlay,
  onTogglePositioningMode,
}: {
  snapshot: KeyboardSnapshot;
  onAdvance: () => void;
  onDragOverlay: () => void;
  onResizeOverlay: () => void;
  onTogglePositioningMode: () => void;
}) {
  const layerName =
    snapshot.keymap.layers.find(
      (layer) => layer.id === snapshot.runtime_state.layer_stack[0]?.layer_id,
    )?.name ?? "Unknown";
  const health = snapshot.runtime_state.backend_health[0];
  const activeLayer = snapshot.runtime_state.layer_stack[0] ?? null;
  const bounds = useMemo(() => layoutBounds(snapshot), [snapshot]);
  const overlayOpacity = clampOpacity(snapshot.overlay_window.display_targeting.opacity);
  const overlayStyle = useMemo(
    () => ({
      opacity: overlayOpacity,
      ...visualStyleCssVariables(snapshot.visual_style.colors),
    }),
    [overlayOpacity, snapshot.visual_style.colors],
  );
  const topLayerId = activeLayer?.layer_id ?? null;

  return (
    <section
      className="overlay-surface"
      style={overlayStyle}
      aria-label="Keyboard overlay"
    >
      <div className="overlay-status">
        <div>
          <span>{layerName}</span>
          <strong>{activeLayer?.confidence.level ?? "low"} confidence</strong>
          <small>{activeLayer?.confidence.reason ?? "No State Confidence reason reported"}</small>
        </div>
        <div>
          <span>{health?.state ?? "unknown"}</span>
          <strong>{snapshot.overlay_window.click_through ? "click-through" : "positioning"}</strong>
          <small>{health?.message ?? "No Backend Health reported"}</small>
        </div>
        <div>
          <span>{snapshot.overlay_window.visibility}</span>
          <strong>{snapshot.overlay_window.visible ? "visible" : "hidden"}</strong>
        </div>
      </div>

      <div className="keyboard-plane" style={{ aspectRatio: `${bounds.width} / ${bounds.height}` }}>
        {snapshot.physical_layout.keys.map((physicalKey) => {
          const effective = snapshot.effective_keys.find((key) => key.key_id === physicalKey.id);
          if (!effective) return null;
          const topActiveLayer =
            topLayerId !== null &&
            snapshot.runtime_state.layer_stack.length > 1 &&
            effective.source_layer_id === topLayerId &&
            !effective.inherited;
          return (
            <Keycap
              key={physicalKey.id}
              effective={effective}
              density={snapshot.visual_style.density}
              topActiveLayer={topActiveLayer}
              pressed={snapshot.runtime_state.pressed_keys.includes(physicalKey.id)}
              style={{
                left: `${((physicalKey.geometry.x - bounds.x) / bounds.width) * 100}%`,
                top: `${((physicalKey.geometry.y - bounds.y) / bounds.height) * 100}%`,
                width: `${(physicalKey.geometry.width / bounds.width) * 100}%`,
                height: `${(physicalKey.geometry.height / bounds.height) * 100}%`,
                transform: `rotate(${physicalKey.geometry.rotation}deg)`,
              }}
            />
          );
        })}
      </div>

      <div className="overlay-controls">
        <button onClick={onAdvance}>
          <RadioTower size={16} />
          Event
        </button>
        <button onClick={onTogglePositioningMode}>
          <Move size={16} />
          {snapshot.overlay_window.positioning_mode ? "Lock" : "Place"}
        </button>
        {snapshot.overlay_window.positioning_mode ? (
          <>
            <button onPointerDown={onDragOverlay}>
              <Move size={16} />
              Drag
            </button>
            <button onPointerDown={onResizeOverlay}>
              <Maximize2 size={16} />
              Resize
            </button>
          </>
        ) : null}
      </div>
    </section>
  );
}

function Keycap({
  effective,
  density,
  topActiveLayer,
  pressed,
  style,
}: {
  effective: EffectiveKey;
  density: StyleDensity;
  topActiveLayer: boolean;
  pressed: boolean;
  style: CSSProperties;
}) {
  const primary =
    effective.legend.slots.find((slot) => slot.slot === "primary")?.text ??
    effective.semantic.label ??
    effective.raw.value;
  const secondarySlots = visibleLegendSlots(effective, density);

  return (
    <button
      className={`keycap density-${density} ${pressed ? "pressed" : ""} ${
        topActiveLayer ? "top-active-layer" : ""
      } ${
        effective.inherited ? "inherited" : ""
      }`}
      style={style}
      aria-label={`${effective.key_id} ${primary}`}
      title={effective.raw.value}
    >
      <span>{primary}</span>
      {secondarySlots.length > 0 ? (
        <div className="legend-slots">
          {secondarySlots.map((slot) => (
            <small
              className={`legend-slot slot-${slot.slot}`}
              data-slot-kind={slot.slot}
              key={`${slot.slot}-${slot.text}`}
            >
              {slot.text}
            </small>
          ))}
        </div>
      ) : null}
      {effective.inherited ? <i>inherited</i> : null}
    </button>
  );
}

function visibleLegendSlots(effective: EffectiveKey, density: StyleDensity) {
  const secondarySlots = effective.legend.slots.filter((slot) => slot.slot !== "primary");
  if (density === "compact") return [];
  if (density === "standard") return secondarySlots.slice(0, 1);
  return secondarySlots;
}

export function kanataTcpSettingsFromSnapshot(snapshot: KeyboardSnapshot) {
  const config = snapshot.backends.find((backend) => backend.id === "kanata-tcp")?.config;
  return config?.kind === "kanata-tcp"
    ? {
        host: config.host,
        port: String(config.port),
      }
    : null;
}

function visualStyleCssVariables(
  colors: KeyboardSnapshot["visual_style"]["colors"],
): CSSProperties {
  const variables: Record<string, string> = {};
  if (colors.keycap_background) {
    variables["--keyplane-keycap-background"] = colors.keycap_background;
  }
  if (colors.keycap_text) {
    variables["--keyplane-keycap-text"] = colors.keycap_text;
  }
  if (colors.keycap_border) {
    variables["--keyplane-keycap-border"] = colors.keycap_border;
  }
  if (colors.modifier_accent) {
    variables["--keyplane-modifier-accent"] = colors.modifier_accent;
  }
  if (colors.overlay_background) {
    variables["--keyplane-overlay-background"] = colors.overlay_background;
  }
  return variables as CSSProperties;
}

function SourceInspector({
  snapshot,
  onPromote,
}: {
  snapshot: KeyboardSnapshot;
  onPromote: (conflict: SourceConflict, sourceId: string) => void;
}) {
  const sourceName = (sourceId: string) =>
    snapshot.sources.find((source) => source.id === sourceId)?.name ?? sourceId;
  const layerName = (layerId: string) =>
    snapshot.keymap.layers.find((layer) => layer.id === layerId)?.name ?? layerId;
  const transparentRows = transparentEntryRows(snapshot);

  return (
    <section className="inspector-view">
      <div className="inspector-grid">
        <div>
          <section>
            <h2>Active Profile</h2>
            <article className="list-row">
              <strong>{snapshot.profile_name}</strong>
              <span className="badge">{snapshot.profile_id}</span>
              <p>Keyboard ID: {snapshot.keyboard_id}</p>
              <p>Style ID: {snapshot.visual_style.id}</p>
            </article>
          </section>

          <section>
            <h2>Sources</h2>
            {snapshot.sources.map((source) => (
              <article className="list-row" key={source.id}>
                <strong>{source.name}</strong>
                <span className="badge">{source.authority}</span>
                <p>{source.kind}</p>
              </article>
            ))}
          </section>

          <section>
            <h2>Backends</h2>
            {snapshot.backends.map((backend) => (
              <article className="list-row" key={backend.id}>
                <strong>{backend.name}</strong>
                <span className="badge">{backend.health.state}</span>
                <p>{backend.health.message}</p>
                <p>{backend.capabilities.join(", ")}</p>
              </article>
            ))}
          </section>

          <section>
            <h2>Layer Stack</h2>
            {snapshot.runtime_state.layer_stack.length === 0 ? <p>No active layers.</p> : null}
            {snapshot.runtime_state.layer_stack.map((activation, index) => (
              <article className="list-row" key={`${activation.layer_id}-${index}`}>
                <strong>
                  {index + 1}. {layerName(activation.layer_id)}
                </strong>
                <span className="badge">
                  {index === 0 ? "top precedence" : "lower precedence"}
                </span>
                <p>
                  {activation.layer_id} - {activation.kind}
                </p>
                <p>{activation.confidence.level} confidence</p>
                <p>{activation.confidence.reason}</p>
              </article>
            ))}
          </section>

          <section>
            <h2>Precedence</h2>
            {snapshot.source_precedence.map((rule) => (
              <article className="list-row" key={rule.field_scope}>
                <strong>{rule.field_scope}</strong>
                <p>{rule.source_order.join(" -> ")}</p>
              </article>
            ))}
          </section>
        </div>

        <div>
          <section>
            <h2>Source Conflicts</h2>
            {snapshot.source_conflicts.length === 0 ? <p>No conflicts for the selected values.</p> : null}
            {snapshot.source_conflicts.map((conflict) => (
              <article className="list-row" key={conflict.field_path}>
                <strong>{conflict.field_path}</strong>
                <span className="badge">{conflict.selected_source_id}</span>
                <ConflictCandidateRows
                  conflict={conflict}
                  sourceProvenance={snapshot.source_provenance}
                  onPromote={onPromote}
                />
              </article>
            ))}
          </section>

          <section>
            <h2>Source Provenance</h2>
            {snapshot.source_provenance.length === 0 ? <p>No provenance records.</p> : null}
            {snapshot.source_provenance.map((sourceRef) => (
              <article className="list-row" key={`${sourceRef.source_id}-${sourceRef.field_path}`}>
                <strong>{sourceRef.field_path}</strong>
                <span className="badge">{sourceName(sourceRef.source_id)}</span>
                {sourceRef.raw ? <p className="provenance-raw">{sourceRef.raw}</p> : null}
              </article>
            ))}
          </section>

          <section>
            <h2>Transparent Entries</h2>
            {transparentRows.length === 0 ? <p>No active transparent entries.</p> : null}
            {transparentRows.map((row) => (
              <article
                className="list-row"
                key={`${row.transparentLayerId}-${row.keyId}-${row.rawValue}`}
              >
                <strong>{row.keyId}</strong>
                <span className="badge">{row.transparentLayerId}</span>
                <p>{row.rawValue}</p>
                <p>
                  inherits {row.effectiveLabel} from {row.inheritedLayerId}
                </p>
                <p>{row.provenanceFieldPath}</p>
              </article>
            ))}
          </section>

          <section>
            <h2>User Overrides</h2>
            {snapshot.user_overrides.length === 0 ? <p>No active overrides.</p> : null}
            {snapshot.user_overrides.map((override) => (
              <article className="list-row" key={override.field_path}>
                <strong>{override.field_path}</strong>
                <span className="badge">{override.value}</span>
                <p>{override.reason}</p>
              </article>
            ))}
          </section>
        </div>
      </div>
    </section>
  );
}

type TransparentEntryRow = {
  keyId: string;
  transparentLayerId: string;
  rawValue: string;
  inheritedLayerId: string;
  effectiveLabel: string;
  provenanceFieldPath: string;
};

function transparentEntryRows(snapshot: KeyboardSnapshot): TransparentEntryRow[] {
  const layersById = new Map(snapshot.keymap.layers.map((layer) => [layer.id, layer]));

  return snapshot.effective_keys.flatMap((effective) => {
    if (!effective.inherited) return [];

    const rows: TransparentEntryRow[] = [];
    for (const activation of snapshot.runtime_state.layer_stack) {
      if (activation.layer_id === effective.source_layer_id) break;

      const layer = layersById.get(activation.layer_id);
      if (!layer) continue;
      const action = layer.actions.find((candidate) => candidate.key_id === effective.key_id);
      if (action?.semantic.kind !== "transparent") continue;

      rows.push({
        keyId: effective.key_id,
        transparentLayerId: layer.id,
        rawValue: action.raw.value,
        inheritedLayerId: effective.source_layer_id,
        effectiveLabel:
          effective.legend.slots.find((slot) => slot.slot === "primary")?.text ??
          effective.semantic.label ??
          effective.raw.value,
        provenanceFieldPath: action.provenance.field_path,
      });
    }

    return rows;
  });
}

function ConflictCandidateRows({
  conflict,
  sourceProvenance,
  onPromote,
}: {
  conflict: SourceConflict;
  sourceProvenance: SourceRef[];
  onPromote?: (conflict: SourceConflict, sourceId: string) => void;
}) {
  return (
    <div className="conflict-candidates">
      {conflict.candidates.map((candidate) => {
        const provenance = provenanceForConflictCandidate(
          sourceProvenance,
          conflict.field_path,
          candidate.source_id,
        );
        return (
          <div
            className={`candidate-row ${candidate.selected ? "selected" : ""}`}
            key={candidate.source_id}
          >
            <div>
              <strong>{candidate.source_id}</strong>
              <span>{candidate.value}</span>
              {provenance.map((sourceRef) =>
                sourceRef.raw ? (
                  <p
                    className="provenance-raw"
                    key={`${sourceRef.source_id}-${sourceRef.field_path}-${sourceRef.raw}`}
                  >
                    {sourceRef.raw}
                  </p>
                ) : null,
              )}
            </div>
            {candidate.selected ? (
              <span className="badge">Selected</span>
            ) : onPromote ? (
              <button
                className="promote-button"
                onClick={() => onPromote(conflict, candidate.source_id)}
              >
                <Check size={15} />
                Promote
              </button>
            ) : (
              <span className="badge">Candidate</span>
            )}
          </div>
        );
      })}
    </div>
  );
}

function provenanceForConflictCandidate(
  sourceProvenance: SourceRef[],
  conflictFieldPath: string,
  sourceId: string,
) {
  return sourceProvenance.filter(
    (sourceRef) =>
      sourceRef.source_id === sourceId &&
      (sourceRef.field_path === conflictFieldPath ||
        conflictFieldPath.startsWith(`${sourceRef.field_path} `) ||
        sourceRef.field_path.startsWith(`${conflictFieldPath} `)),
  );
}

type ImportDiffRow = {
  label: string;
  current: string;
  preview: string;
  changed: boolean;
};

export function buildImportDiffRows(
  activeSnapshot: KeyboardSnapshot,
  candidate: ImportCandidate,
): ImportDiffRow[] {
  const profile = candidate.preview_profile;
  const rows = [
    {
      label: "Physical Keys",
      current: String(activeSnapshot.physical_layout.keys.length),
      preview: String(profile.physical_layout.keys.length),
    },
    {
      label: "Layers",
      current: String(activeSnapshot.keymap.layers.length),
      preview: String(profile.keymap.layers.length),
    },
    {
      label: "Sources",
      current: String(activeSnapshot.sources.length),
      preview: String(profile.sources.length),
    },
    {
      label: "Keyboard ID",
      current: activeSnapshot.keyboard_id,
      preview: profile.keyboard_id,
    },
    {
      label: "Source Provenance",
      current: String(activeSnapshot.source_provenance.length),
      preview: String(profile.source_provenance.length),
    },
    {
      label: "Backends",
      current: String(activeSnapshot.backends.length),
      preview: String(profile.runtime_backends.length),
    },
    {
      label: "Visual Style",
      current: visualStyleSummary(activeSnapshot.visual_style),
      preview: visualStyleSummary(profile.visual_style),
    },
    {
      label: "Style ID",
      current: activeSnapshot.visual_style.id,
      preview: profile.visual_style.id,
    },
    {
      label: "Fallback Layout",
      current: activeSnapshot.physical_layout.fallback ? "yes" : "no",
      preview: profile.physical_layout.fallback ? "yes" : "no",
    },
  ];

  return rows.map((row) => ({
    ...row,
    changed: row.current !== row.preview,
  }));
}

function visualStyleSummary(style: KeyboardSnapshot["visual_style"]) {
  const colorTokenCount = Object.values(style.colors).filter(Boolean).length;
  const referenceId = style.id || style.variant_id;
  const styleLabel =
    referenceId === style.variant_id ? style.variant_id : `${referenceId} (${style.variant_id})`;
  return colorTokenCount === 0
    ? styleLabel
    : `${styleLabel} + ${colorTokenCount} color tokens`;
}

export function ImportReview({
  activeSnapshot,
  candidate,
  error,
  onCommit,
  onPromote,
}: {
  activeSnapshot: KeyboardSnapshot;
  candidate: ImportCandidate | null;
  error: string | null;
  onCommit: (candidate: ImportCandidate) => void;
  onPromote?: (conflict: SourceConflict, sourceId: string) => void;
}) {
  if (error) return <section className="empty-state">{error}</section>;
  if (!candidate) {
    return (
      <section className="empty-state">
        Select a `.vil`, QMK info JSON, `.keymap`, style JSON, or companion JSON export to preview a Best-Effort Import Candidate.
      </section>
    );
  }

  return (
    <section className="import-view">
      <header>
        <div>
          <span className="badge">Best-Effort Preview</span>
          <h2>{candidate.preview_profile.name}</h2>
        </div>
        <button onClick={() => onCommit(candidate)}>
          <Check size={17} />
          Use Preview
        </button>
      </header>
      <div className="metric-strip">
        <strong>{candidate.summary.imported_keys} keys</strong>
        <strong>{candidate.summary.imported_layers} layers</strong>
        <strong>{candidate.summary.preserved_sections.length} preserved sections</strong>
      </div>
      <section className="import-diff">
        <h3>Profile Diff</h3>
        <div className="diff-grid">
          {buildImportDiffRows(activeSnapshot, candidate).map((row) => (
            <article className={`diff-row ${row.changed ? "changed" : ""}`} key={row.label}>
              <span>{row.label}</span>
              <strong>
                {row.current} -&gt; {row.preview}
              </strong>
            </article>
          ))}
        </div>
      </section>
      <p>{candidate.preview_profile.runtime_backends[0]?.health.message}</p>
      <div className="preserved-sections">
        {candidate.summary.preserved_sections.map((section) => (
          <span key={section}>{section}</span>
        ))}
      </div>
      {candidate.conflicts.length > 0 ? (
        <section className="import-conflicts">
          <h3>Source Conflicts</h3>
          {candidate.conflicts.map((conflict) => (
            <article className="list-row" key={conflict.field_path}>
              <strong>{conflict.field_path}</strong>
              <span className="badge">{conflict.selected_source_id}</span>
              <ConflictCandidateRows
                conflict={conflict}
                sourceProvenance={candidate.preview_profile.source_provenance}
                onPromote={onPromote}
              />
            </article>
          ))}
        </section>
      ) : null}
    </section>
  );
}

function SettingsView({
  density,
  hostPermissionMessage,
  hostPermissionState,
  launchAtLogin,
  kanataHealthState,
  kanataHost,
  kanataPort,
  overlayVisible,
  overlayVisibility,
  sentinelKeysEnabled,
  onDensityChange,
  onHostPermissionRefresh,
  onHostPermissionRequest,
  onLaunchAtLoginChange,
  onKanataConnect,
  onKanataDisconnect,
  onKanataHostChange,
  onKanataPortChange,
  onOverlayVisibilityChange,
  onOverlayVisibleChange,
  onSentinelKeysChange,
}: {
  density: StyleDensity;
  hostPermissionMessage: string | null;
  hostPermissionState: string | null;
  launchAtLogin: boolean | null;
  kanataHealthState: string | null;
  kanataHost: string;
  kanataPort: string;
  overlayVisible: boolean;
  overlayVisibility: VisibilityPolicy;
  sentinelKeysEnabled: boolean | null;
  onDensityChange: (density: StyleDensity) => void;
  onHostPermissionRefresh: () => void;
  onHostPermissionRequest: () => void;
  onLaunchAtLoginChange: (enabled: boolean) => void;
  onKanataConnect: () => void;
  onKanataDisconnect: () => void;
  onKanataHostChange: (host: string) => void;
  onKanataPortChange: (port: string) => void;
  onOverlayVisibilityChange: (visibility: VisibilityPolicy) => void;
  onOverlayVisibleChange: (visible: boolean) => void;
  onSentinelKeysChange: (enabled: boolean) => void;
}) {
  return (
    <section className="settings-view" aria-label="App settings">
      <section>
        <h2>Startup</h2>
        <label className="setting-toggle">
          <span>Launch at login</span>
          <input
            type="checkbox"
            checked={launchAtLogin === true}
            disabled={launchAtLogin === null}
            onChange={(event) => onLaunchAtLoginChange(event.currentTarget.checked)}
          />
          <span className="badge">
            {launchAtLogin === null ? "unavailable" : launchAtLogin ? "enabled" : "disabled"}
          </span>
        </label>
      </section>
      <section>
        <h2>Visual Style</h2>
        <div className="density-options" aria-label="Visual style density">
          {(["compact", "standard", "rich"] as const).map((option) => (
            <button
              className={density === option ? "active" : ""}
              key={option}
              onClick={() => onDensityChange(option)}
              type="button"
            >
              {option}
            </button>
          ))}
        </div>
      </section>
      <section>
        <h2>Overlay Window</h2>
        <div className="density-options" aria-label="Overlay visibility policy">
          {(["pinned", "manual-toggle", "fade"] as const).map((option) => (
            <button
              className={overlayVisibility === option ? "active" : ""}
              key={option}
              onClick={() => onOverlayVisibilityChange(option)}
              type="button"
            >
              {option}
            </button>
          ))}
        </div>
        <label className="setting-toggle">
          <span>Overlay visible</span>
          <input
            type="checkbox"
            checked={overlayVisible}
            onChange={(event) => onOverlayVisibleChange(event.currentTarget.checked)}
          />
          <span className="badge">{overlayVisible ? "visible" : "hidden"}</span>
          {overlayVisible ? <Eye size={17} /> : <EyeOff size={17} />}
        </label>
      </section>
      <section>
        <h2>Protocol Backends</h2>
        <div className="backend-connect">
          <label>
            <span>Kanata host</span>
            <input
              aria-label="Kanata host"
              value={kanataHost}
              onChange={(event) => onKanataHostChange(event.currentTarget.value)}
            />
          </label>
          <label>
            <span>Kanata port</span>
            <input
              aria-label="Kanata port"
              inputMode="numeric"
              value={kanataPort}
              onChange={(event) => onKanataPortChange(event.currentTarget.value)}
            />
          </label>
          <button type="button" onClick={onKanataConnect}>
            <RadioTower size={17} />
            Connect Kanata
          </button>
          <button type="button" onClick={onKanataDisconnect}>
            <RadioTower size={17} />
            Stop Kanata
          </button>
          <span className="badge">{kanataHealthState ?? "unknown"}</span>
        </div>
        <label className="setting-toggle">
          <span>
            <Keyboard size={17} />
            Sentinel Keys
          </span>
          <input
            type="checkbox"
            checked={sentinelKeysEnabled === true}
            disabled={sentinelKeysEnabled === null}
            onChange={(event) => onSentinelKeysChange(event.currentTarget.checked)}
          />
          <span className="badge">
            {sentinelKeysEnabled === null ? "unavailable" : sentinelKeysEnabled ? "enabled" : "disabled"}
          </span>
        </label>
        <div className="backend-connect permission-controls">
          <span className="backend-label">
            <ShieldAlert size={17} />
            Host Input Permissions
          </span>
          <button type="button" onClick={onHostPermissionRefresh}>
            <ShieldCheck size={17} />
            Check
          </button>
          <button type="button" onClick={onHostPermissionRequest}>
            <ShieldAlert size={17} />
            Request
          </button>
          <span className="badge">{hostPermissionState ?? "unknown"}</span>
        </div>
        {hostPermissionMessage ? (
          <p className="settings-status">{hostPermissionMessage}</p>
        ) : null}
      </section>
    </section>
  );
}

function layoutBounds(snapshot: KeyboardSnapshot) {
  const right = Math.max(
    ...snapshot.physical_layout.keys.map((key) => key.geometry.x + key.geometry.width),
  );
  const bottom = Math.max(
    ...snapshot.physical_layout.keys.map((key) => key.geometry.y + key.geometry.height),
  );
  const left = Math.min(...snapshot.physical_layout.keys.map((key) => key.geometry.x));
  const top = Math.min(...snapshot.physical_layout.keys.map((key) => key.geometry.y));

  return {
    x: left,
    y: top,
    width: right - left,
    height: bottom - top,
  };
}

function clampOpacity(value: number) {
  if (!Number.isFinite(value)) return 1;
  return Math.min(Math.max(value, 0), 1);
}

function selectedKeyPeekDeviceIndex(
  devices: KeyPeekDiscoveredDevice[],
  vid: string,
  pid: string,
) {
  const index = devices.findIndex((device) => device.vid === vid && device.pid === pid);
  return index >= 0 ? String(index) : "";
}

function downloadTextFile(filename: string, contents: string) {
  const url = URL.createObjectURL(new Blob([contents], { type: "text/plain;charset=utf-8" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  URL.revokeObjectURL(url);
}

export default App;
