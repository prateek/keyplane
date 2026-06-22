// Renders a resolved keyboard from per-key coordinate geometry (ADR 0028).

import type { CSSProperties } from "react";
import type { ResolvedKey, VisualStyle } from "../types";
import { collapseLegend } from "../overlayState";

const UNIT = 48; // px per keycap unit
const GAP = 4;

interface KeyboardProps {
  keys: ResolvedKey[];
  extent: [number, number];
  style: VisualStyle;
  topLayer?: string;
  pressed?: Set<string>;
}

export function Keyboard({ keys, extent, style, topLayer, pressed }: KeyboardProps) {
  const [w, h] = extent;
  return (
    <div
      className="keyboard"
      style={{ width: w * UNIT, height: h * UNIT, opacity: style.opacity }}
    >
      {keys.map((key) => (
        <Keycap
          key={key.key}
          rk={key}
          style={style}
          onTop={key.source_layer === topLayer}
          isPressed={pressed?.has(key.key) ?? false}
        />
      ))}
    </div>
  );
}

interface KeycapProps {
  rk: ResolvedKey;
  style: VisualStyle;
  onTop: boolean;
  isPressed: boolean;
}

function Keycap({ rk, style, onTop, isPressed }: KeycapProps) {
  const g = rk.geometry;
  const transform =
    g.rotation && g.rotation_origin
      ? `rotate(${g.rotation}deg)`
      : undefined;
  const classes = ["keycap"];
  if (rk.inherited) classes.push("inherited");
  if (onTop) classes.push("on-top");
  if (isPressed) classes.push("pressed");
  if (rk.effective.kind === "none") classes.push("blank");

  const css: CSSProperties = {
    left: g.x * UNIT + GAP / 2,
    top: g.y * UNIT + GAP / 2,
    width: g.w * UNIT - GAP,
    height: g.h * UNIT - GAP,
    transform,
    transformOrigin: g.rotation_origin
      ? `${(g.rotation_origin[0] - g.x) * UNIT}px ${(g.rotation_origin[1] - g.y) * UNIT}px`
      : undefined,
    background: style.keycap_color,
    color: style.text_color,
    borderColor: onTop ? style.accent : undefined,
  };

  return (
    <div className={classes.join(" ")} style={css} data-key={rk.key}>
      {style.variant === "minimal" ? (
        <span className="legend-primary">{collapseLegend(rk)}</span>
      ) : (
        <DetailedLegend rk={rk} />
      )}
      {rk.inherited && style.show_inherited_indicator ? (
        <span className="inherited-dot" title={`inherited from ${rk.source_layer}`} />
      ) : null}
    </div>
  );
}

function DetailedLegend({ rk }: { rk: ResolvedKey }) {
  const l = rk.legend;
  return (
    <>
      {l.hold ? <span className="legend-hold">{l.hold}</span> : null}
      <span className="legend-primary">{l.primary ?? l.tap ?? ""}</span>
      {l.tap && l.primary ? <span className="legend-tap">{l.tap}</span> : null}
      {l.layer ? <span className="legend-layer">{l.layer}</span> : null}
      {l.shifted ? <span className="legend-shifted">{l.shifted}</span> : null}
    </>
  );
}
