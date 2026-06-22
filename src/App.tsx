import {
  Check,
  Crosshair,
  FileJson,
  Layers3,
  Move,
  PanelLeft,
  RadioTower,
  ShieldAlert,
  ShieldCheck,
  Upload,
} from "lucide-react";
import type { CSSProperties } from "react";
import { useEffect, useMemo, useState } from "react";
import "./App.css";
import type {
  EffectiveKey,
  ImportCandidate,
  KeyboardSnapshot,
  RuntimeEvent,
  SourceConflict,
} from "./domain";
import { navLayerEvent } from "./fixtures";
import { applyRuntimeEvent, promoteSourceCandidate } from "./state";
import {
  importVialFile,
  loadFakeRuntimeEvents,
  loadInitialSnapshot,
  setOverlayPositioningMode,
} from "./tauriClient";

type View = "overlay" | "inspector" | "import";

function App() {
  const overlayOnly = window.location.hash === "#/overlay";
  const [view, setView] = useState<View>(overlayOnly ? "overlay" : "overlay");
  const [snapshot, setSnapshot] = useState<KeyboardSnapshot | null>(null);
  const [events, setEvents] = useState<RuntimeEvent[]>([]);
  const [eventIndex, setEventIndex] = useState(0);
  const [importCandidate, setImportCandidate] = useState<ImportCandidate | null>(null);
  const [importError, setImportError] = useState<string | null>(null);

  useEffect(() => {
    void loadInitialSnapshot().then(setSnapshot);
    void loadFakeRuntimeEvents().then(setEvents);
  }, []);

  const activeLayer = snapshot?.runtime_state.layer_stack[0];
  const health = snapshot?.runtime_state.backend_health ?? [];
  const topHealth = health[0];

  async function togglePositioningMode() {
    if (!snapshot) return;
    const enabled = !snapshot.overlay_window.positioning_mode;
    await setOverlayPositioningMode(enabled);
    setSnapshot({
      ...snapshot,
      overlay_window: {
        ...snapshot.overlay_window,
        positioning_mode: enabled,
        click_through: !enabled,
      },
    });
  }

  function advanceFakeEvent() {
    if (!snapshot) return;
    const event = events[eventIndex] ?? navLayerEvent;
    setSnapshot(applyRuntimeEvent(snapshot, event));
    setEventIndex((index) => (index + 1) % Math.max(events.length, 1));
  }

  function promoteSourceValue(conflict: SourceConflict, sourceId: string) {
    setSnapshot((current) => (current ? promoteSourceCandidate(current, conflict, sourceId) : current));
  }

  async function handleImport(file: File | null) {
    if (!file) return;
    setImportError(null);
    try {
      const contents = await file.text();
      setImportCandidate(await importVialFile(contents));
      setView("import");
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
          </div>
          <div className="toolbar-actions">
            <button onClick={advanceFakeEvent}>
              <RadioTower size={17} />
              Fake event
            </button>
            <button onClick={togglePositioningMode}>
              {snapshot.overlay_window.positioning_mode ? <Crosshair size={17} /> : <Move size={17} />}
              {snapshot.overlay_window.positioning_mode ? "Done" : "Position"}
            </button>
            <label className="file-button">
              <FileJson size={17} />
              Vial file
              <input
                type="file"
                accept=".vil,application/json"
                onChange={(event) => void handleImport(event.currentTarget.files?.[0] ?? null)}
              />
            </label>
          </div>
        </header>

        {view === "overlay" ? (
          <OverlaySurface
            snapshot={snapshot}
            onAdvance={advanceFakeEvent}
            onTogglePositioningMode={togglePositioningMode}
          />
        ) : null}
        {view === "inspector" ? (
          <SourceInspector snapshot={snapshot} onPromote={promoteSourceValue} />
        ) : null}
        {view === "import" ? (
          <ImportReview candidate={importCandidate} error={importError} />
        ) : null}
      </section>
    </main>
  );
}

function OverlaySurface({
  snapshot,
  onAdvance,
  onTogglePositioningMode,
}: {
  snapshot: KeyboardSnapshot;
  onAdvance: () => void;
  onTogglePositioningMode: () => void;
}) {
  const layerName =
    snapshot.keymap.layers.find(
      (layer) => layer.id === snapshot.runtime_state.layer_stack[0]?.layer_id,
    )?.name ?? "Unknown";
  const health = snapshot.runtime_state.backend_health[0];
  const bounds = useMemo(() => layoutBounds(snapshot), [snapshot]);

  return (
    <section className="overlay-surface" aria-label="Keyboard overlay">
      <div className="overlay-status">
        <div>
          <span>{layerName}</span>
          <strong>{snapshot.runtime_state.layer_stack[0]?.confidence.level ?? "low"} confidence</strong>
        </div>
        <div>
          <span>{health?.state ?? "unknown"}</span>
          <strong>{snapshot.overlay_window.click_through ? "click-through" : "positioning"}</strong>
        </div>
      </div>

      <div className="keyboard-plane" style={{ aspectRatio: `${bounds.width} / ${bounds.height}` }}>
        {snapshot.physical_layout.keys.map((physicalKey) => {
          const effective = snapshot.effective_keys.find((key) => key.key_id === physicalKey.id);
          if (!effective) return null;
          return (
            <Keycap
              key={physicalKey.id}
              effective={effective}
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
      </div>
    </section>
  );
}

function Keycap({
  effective,
  pressed,
  style,
}: {
  effective: EffectiveKey;
  pressed: boolean;
  style: CSSProperties;
}) {
  const primary = effective.legend.slots.find((slot) => slot.slot === "primary")?.text;
  const hint = effective.legend.slots.find((slot) => slot.slot !== "primary")?.text;

  return (
    <button
      className={`keycap ${pressed ? "pressed" : ""} ${effective.inherited ? "inherited" : ""}`}
      style={style}
      aria-label={`${effective.key_id} ${primary}`}
      title={effective.raw.value}
    >
      <span>{primary}</span>
      {hint ? <small>{hint}</small> : null}
      {effective.inherited ? <i>inherited</i> : null}
    </button>
  );
}

function SourceInspector({
  snapshot,
  onPromote,
}: {
  snapshot: KeyboardSnapshot;
  onPromote: (conflict: SourceConflict, sourceId: string) => void;
}) {
  return (
    <section className="inspector-view">
      <div className="inspector-grid">
        <div>
          <section>
            <h2>Backends</h2>
            {snapshot.backends.map((backend) => (
              <article className="list-row" key={backend.id}>
                <strong>{backend.name}</strong>
                <span className="badge">{backend.health.state}</span>
                <p>{backend.capabilities.join(", ")}</p>
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
                <div className="conflict-candidates">
                  {conflict.candidates.map((candidate) => (
                    <div
                      className={`candidate-row ${candidate.selected ? "selected" : ""}`}
                      key={candidate.source_id}
                    >
                      <div>
                        <strong>{candidate.source_id}</strong>
                        <span>{candidate.value}</span>
                      </div>
                      {candidate.selected ? (
                        <span className="badge">Selected</span>
                      ) : (
                        <button
                          className="promote-button"
                          onClick={() => onPromote(conflict, candidate.source_id)}
                        >
                          <Check size={15} />
                          Promote
                        </button>
                      )}
                    </div>
                  ))}
                </div>
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

function ImportReview({
  candidate,
  error,
}: {
  candidate: ImportCandidate | null;
  error: string | null;
}) {
  if (error) return <section className="empty-state">{error}</section>;
  if (!candidate) {
    return (
      <section className="empty-state">
        Select a `.vil` JSON export to preview a Best-Effort Import Candidate.
      </section>
    );
  }

  return (
    <section className="import-view">
      <header>
        <span className="badge">Best-Effort Preview</span>
        <h2>{candidate.preview_profile.name}</h2>
      </header>
      <div className="metric-strip">
        <strong>{candidate.summary.imported_keys} keys</strong>
        <strong>{candidate.summary.imported_layers} layers</strong>
        <strong>{candidate.summary.preserved_sections.length} preserved sections</strong>
      </div>
      <p>{candidate.preview_profile.runtime_backends[0]?.health.message}</p>
      <div className="preserved-sections">
        {candidate.summary.preserved_sections.map((section) => (
          <span key={section}>{section}</span>
        ))}
      </div>
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

export default App;
