// The App Window: status, import + Import Review, Source Inspector, the
// hand-editable EDN view, and overlay controls (ADR 0019, 0021, 0024, 0025).

import { useEffect, useState } from "react";
import {
  activeProfileEdn,
  applyProfileEdn,
  commitImport,
  connectKanata,
  connectKeypeek,
  discoverDevices,
  getAutostart,
  getSnapshot,
  importPreview,
  onRuntimeEvent,
  onSnapshot,
  promoteOverride,
  setAutostart,
  setOverlayVisible,
  setPositioningMode,
} from "../api";
import { StatusBar } from "../components/Status";
import { topLayer } from "../overlayState";
import type { ImportReview, KeyboardSnapshot } from "../types";

const FORMATS = [
  { id: "vial", label: "Vial .vil" },
  { id: "zmk", label: "ZMK .keymap" },
  { id: "overkeys", label: "OverKeys config" },
  { id: "keyviz", label: "keyviz style JSON" },
];

export function App() {
  const [snapshot, setSnapshot] = useState<KeyboardSnapshot | null>(null);

  useEffect(() => {
    let unRuntime: (() => void) | undefined;
    let unSnap: (() => void) | undefined;
    getSnapshot().then(setSnapshot).catch(() => undefined);
    onRuntimeEvent((event) => {
      if (event.type === "layer-stack") {
        setSnapshot((p) =>
          p ? { ...p, layer_stack: event.layer_stack, confidence: event.confidence, keys: event.keys } : p,
        );
      } else if (event.type === "backend-health") {
        setSnapshot((p) =>
          p
            ? {
                ...p,
                backends: p.backends.map((b) =>
                  b.backend_id === event.health.backend_id ? event.health : b,
                ),
              }
            : p,
        );
      }
    }).then((u) => (unRuntime = u));
    onSnapshot(setSnapshot).then((u) => (unSnap = u));
    return () => {
      unRuntime?.();
      unSnap?.();
    };
  }, []);

  return (
    <div className="app">
      <header className="app-header">
        <h1>Keyplane</h1>
        <OverlayControls />
      </header>

      {snapshot ? (
        <>
          <section className="panel">
            <h2>{snapshot.keyboard_name ?? "Keyboard"}</h2>
            <StatusBar
              confidence={snapshot.confidence}
              backends={snapshot.backends}
              topLayer={topLayer(snapshot.layer_stack)}
            />
            <p className="muted">
              {snapshot.layers.length} layers · {snapshot.keys.length} keys
            </p>
          </section>
          <DeviceConnect />
          <ImportPanel />
          <EdnEditor />
        </>
      ) : (
        <p className="muted">Loading…</p>
      )}
    </div>
  );
}

function OverlayControls() {
  const [positioning, setPositioning] = useState(false);
  const [visible, setVisible] = useState(true);
  const [autostart, setAutostartState] = useState(false);

  useEffect(() => {
    getAutostart()
      .then(setAutostartState)
      .catch(() => undefined);
  }, []);

  return (
    <div className="overlay-controls">
      <button
        onClick={() => {
          const next = !positioning;
          setPositioning(next);
          setPositioningMode(next).catch(() => undefined);
        }}
      >
        {positioning ? "Lock overlay" : "Position overlay"}
      </button>
      <button
        onClick={() => {
          const next = !visible;
          setVisible(next);
          setOverlayVisible(next).catch(() => undefined);
        }}
      >
        {visible ? "Hide overlay" : "Show overlay"}
      </button>
      <button
        onClick={() => {
          const next = !autostart;
          setAutostartState(next);
          setAutostart(next).catch(() => undefined);
        }}
      >
        {autostart ? "✓ Launch at login" : "Launch at login"}
      </button>
    </div>
  );
}

function DeviceConnect() {
  const [vid, setVid] = useState("");
  const [pid, setPid] = useState("");
  const [status, setStatus] = useState<string | null>(null);

  const connect = async () => {
    setStatus("connecting…");
    try {
      await connectKeypeek({
        kind: "vial",
        vid: parseInt(vid, 16) || parseInt(vid, 10),
        pid: parseInt(pid, 16) || parseInt(pid, 10),
      });
      setStatus("connected");
    } catch (e) {
      setStatus(String(e));
    }
  };

  const scan = async () => {
    setStatus("scanning…");
    try {
      const devices = await discoverDevices();
      if (devices.length === 0) {
        setStatus("no devices found");
        return;
      }
      const d = devices[0];
      setVid(d.vid.toString(16));
      setPid(d.pid.toString(16));
      setStatus(`found ${devices.length}: ${d.display_name}`);
    } catch (e) {
      setStatus(String(e));
    }
  };

  return (
    <section className="panel">
      <h2>Connect device (KeyPeek)</h2>
      <p className="muted">
        Stream live layer state from a Vial/VIA keyboard via the KeyPeek-derived
        backend. Requires a connected, supported device.
      </p>
      <div className="row">
        <button onClick={scan}>Scan</button>
        <input
          className="hex-input"
          placeholder="VID (hex)"
          value={vid}
          onChange={(e) => setVid(e.target.value)}
        />
        <input
          className="hex-input"
          placeholder="PID (hex)"
          value={pid}
          onChange={(e) => setPid(e.target.value)}
        />
        <button onClick={connect}>Connect Vial</button>
        {status ? <span className="muted">{status}</span> : null}
      </div>
      <ZmkConnect vid={vid} pid={pid} />
      <KanataConnect />
    </section>
  );
}

function ZmkConnect({ vid, pid }: { vid: string; pid: string }) {
  const [port, setPort] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const connect = async () => {
    setStatus("connecting…");
    try {
      await connectKeypeek({
        kind: "zmk",
        vid: parseInt(vid, 16) || parseInt(vid, 10),
        pid: parseInt(pid, 16) || parseInt(pid, 10),
        serialPort: port,
      });
      setStatus("connected");
    } catch (e) {
      setStatus(String(e));
    }
  };
  return (
    <div className="row">
      <span className="muted">ZMK Studio (uses VID/PID above):</span>
      <input
        className="hex-input"
        placeholder="serial port"
        value={port}
        onChange={(e) => setPort(e.target.value)}
      />
      <button onClick={connect}>Connect ZMK</button>
      {status ? <span className="muted">{status}</span> : null}
    </div>
  );
}

function KanataConnect() {
  const [port, setPort] = useState("5829");
  const [status, setStatus] = useState<string | null>(null);
  const connect = async () => {
    setStatus("connecting…");
    try {
      await connectKanata(parseInt(port, 10));
      setStatus("connected");
    } catch (e) {
      setStatus(String(e));
    }
  };
  return (
    <div className="row">
      <span className="muted">Kanata (companion profile + `--port`):</span>
      <input
        className="hex-input"
        placeholder="port"
        value={port}
        onChange={(e) => setPort(e.target.value)}
      />
      <button onClick={connect}>Connect Kanata</button>
      {status ? <span className="muted">{status}</span> : null}
    </div>
  );
}

function ImportPanel() {
  const [format, setFormat] = useState(FORMATS[0].id);
  const [contents, setContents] = useState("");
  const [review, setReview] = useState<ImportReview | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = async (fn: () => Promise<unknown>) => {
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <section className="panel">
      <h2>Import</h2>
      <div className="row">
        <select value={format} onChange={(e) => setFormat(e.target.value)}>
          {FORMATS.map((f) => (
            <option key={f.id} value={f.id}>
              {f.label}
            </option>
          ))}
        </select>
        <button onClick={() => run(async () => setReview(await importPreview(format, contents)))}>
          Preview
        </button>
        <button onClick={() => run(async () => { await commitImport(format, contents); setReview(null); })}>
          Commit
        </button>
      </div>
      <textarea
        className="import-input"
        placeholder="Paste .vil / config / style JSON here"
        value={contents}
        onChange={(e) => setContents(e.target.value)}
      />
      {error ? <p className="error">{error}</p> : null}
      {review ? <ReviewView review={review} /> : null}
    </section>
  );
}

function ReviewView({ review }: { review: ImportReview }) {
  return (
    <div className="review">
      {review.best_effort_preview ? (
        <p className="badge confidence-inferred">Best-Effort Preview</p>
      ) : null}
      <p className="muted">
        {review.additions} additions · {review.conflicts.length} conflicts
      </p>
      {review.notes.map((n, i) => (
        <p key={i} className="muted">
          {n}
        </p>
      ))}
      {review.conflicts.length > 0 ? (
        <table className="conflicts">
          <thead>
            <tr>
              <th>Field</th>
              <th>Current</th>
              <th>Incoming</th>
              <th>Winner</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {review.conflicts.map((c) => (
              <tr key={c.field}>
                <td>{c.field}</td>
                <td>
                  {c.current ? (
                    <span title={`from ${c.current.provenance.kind}`}>{c.current.value}</span>
                  ) : (
                    "—"
                  )}
                </td>
                <td title={`from ${c.incoming.provenance.kind}`}>{c.incoming.value}</td>
                <td>{c.winner.kind}</td>
                <td>
                  <button
                    onClick={() =>
                      promoteOverride(
                        c.field,
                        { source: "qmk", value: c.incoming.value },
                        "promoted from import",
                      ).catch(() => undefined)
                    }
                  >
                    Keep mine
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
    </div>
  );
}

function EdnEditor() {
  const [edn, setEdn] = useState("");
  const [status, setStatus] = useState<string | null>(null);

  const load = async () => {
    setEdn(await activeProfileEdn());
    setStatus("loaded");
  };
  const apply = async () => {
    try {
      await applyProfileEdn(edn);
      setStatus("applied");
    } catch (e) {
      setStatus(String(e));
    }
  };

  return (
    <section className="panel">
      <h2>Profile (EDN)</h2>
      <div className="row">
        <button onClick={() => load().catch((e) => setStatus(String(e)))}>Load</button>
        <button onClick={apply}>Apply</button>
        {status ? <span className="muted">{status}</span> : null}
      </div>
      <textarea
        className="edn-input"
        value={edn}
        onChange={(e) => setEdn(e.target.value)}
        spellCheck={false}
      />
    </section>
  );
}
