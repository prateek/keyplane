// The Overlay Surface: a full-keyboard layer overlay (ADR 0006, 0011).
//
// Loads the initial snapshot, then applies streamed Runtime Events. All
// resolution happens in Rust; this only renders.

import { useEffect, useState } from "react";
import { getSnapshot, onRuntimeEvent, onSnapshot } from "../api";
import { Keyboard } from "../components/Keyboard";
import { StatusBar } from "../components/Status";
import {
  applyEvent,
  initialState,
  topLayer,
  type OverlayState,
} from "../overlayState";

export function Overlay() {
  const [state, setState] = useState<OverlayState | null>(null);

  useEffect(() => {
    let unlistenRuntime: (() => void) | undefined;
    let unlistenSnapshot: (() => void) | undefined;
    let active = true;

    getSnapshot()
      .then((snapshot) => {
        if (active) setState(initialState(snapshot));
      })
      .catch(() => {
        /* App Window may load before the backend; events will refresh us. */
      });

    onRuntimeEvent((event) => {
      setState((prev) => (prev ? applyEvent(prev, event) : prev));
    }).then((un) => {
      unlistenRuntime = un;
    });

    onSnapshot((snapshot) => {
      setState(initialState(snapshot));
    }).then((un) => {
      unlistenSnapshot = un;
    });

    return () => {
      active = false;
      unlistenRuntime?.();
      unlistenSnapshot?.();
    };
  }, []);

  if (!state) {
    return <div className="overlay-loading">Waiting for keyboard…</div>;
  }

  const { snapshot, pressed } = state;
  return (
    <div className="overlay-surface">
      <StatusBar
        confidence={snapshot.confidence}
        backends={snapshot.backends}
        topLayer={topLayer(snapshot.layer_stack)}
      />
      <Keyboard
        keys={snapshot.keys}
        extent={snapshot.extent}
        style={snapshot.style}
        topLayer={topLayer(snapshot.layer_stack)}
        pressed={pressed}
      />
    </div>
  );
}
