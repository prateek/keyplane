// Backend Health + State Confidence indicators (ADR 0023, 0032).

import type { BackendHealth, Confidence, HealthState } from "../types";

export function healthLabel(h: HealthState): string {
  switch (h.state) {
    case "ok":
      return "OK";
    case "permission-missing":
      return `Permission needed: ${h.permission}`;
    case "disconnected":
      return "Disconnected";
    case "stale":
      return "Stale";
    case "unsupported":
      return "Unsupported";
    case "parse-error":
      return "Parse error";
    case "protocol-error":
      return "Protocol error";
  }
}

export function HealthBadge({ backend }: { backend: BackendHealth }) {
  return (
    <span className={`badge health-${backend.health.state}`} title={backend.name}>
      {backend.name}: {healthLabel(backend.health)}
    </span>
  );
}

export function ConfidenceBadge({ confidence }: { confidence: Confidence }) {
  const label =
    confidence === "authoritative"
      ? "Live"
      : confidence === "inferred"
        ? "Inferred"
        : "Unknown";
  return <span className={`badge confidence-${confidence}`}>{label}</span>;
}

export function StatusBar({
  confidence,
  backends,
  topLayer,
}: {
  confidence: Confidence;
  backends: BackendHealth[];
  topLayer?: string;
}) {
  return (
    <div className="status-bar">
      <ConfidenceBadge confidence={confidence} />
      {topLayer ? <span className="badge layer">Layer: {topLayer}</span> : null}
      {backends.map((b) => (
        <HealthBadge key={b.backend_id} backend={b} />
      ))}
    </div>
  );
}
