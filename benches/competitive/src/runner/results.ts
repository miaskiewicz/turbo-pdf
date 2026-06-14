// Result schema written to results/*.json and rendered into RESULTS.md. Every
// metric is keyed by (engine, workload) so public claims map 1:1 to a workload id
// (claim-gating, AC-10.13).

import type { Availability, Footprint, WorkloadId } from "../types.ts";
import type { Summary } from "./stats.ts";

/** Per (engine × workload) measured cell. */
export interface Cell {
  readonly engine: string;
  readonly workload: WorkloadId;
  /** null when the engine was skipped or errored for this workload. */
  readonly cold: Summary | null;
  readonly warm: Summary | null;
  /** Engine/process init cost isolated from render (cold runs), ms. */
  readonly initMs: Summary | null;
  /** Output PDF size in bytes (last successful render). */
  readonly pdfBytes: number | null;
  /** Peak RSS observed during the measured runs, bytes. */
  readonly peakRssBytes: number | null;
  /** documents/sec at concurrency = core count (throughput probe). */
  readonly throughputPerSec: number | null;
  /** PNG similarity vs the reference engine, 0..1 (filled by compare/). */
  readonly similarity: number | null;
  /** Set when the engine could not produce this workload. */
  readonly error: string | null;
}

/** Per-engine metadata gathered once. */
export interface EngineInfo {
  readonly id: string;
  readonly kind: string;
  readonly availability: Availability;
  readonly footprint: Footprint;
}

/** Machine + run context, recorded so numbers are "on this machine" (AC-10.11). */
export interface RunContext {
  readonly date: string;
  readonly os: string;
  readonly arch: string;
  readonly cpuModel: string;
  readonly cores: number;
  readonly totalMemBytes: number;
  readonly nodeVersion: string;
  readonly runs: number;
  readonly warmup: number;
  /** Reference engine id used for PNG equivalence. */
  readonly referenceEngine: string;
}

/** Top-level results artifact. */
export interface ResultsDoc {
  readonly context: RunContext;
  readonly engines: readonly EngineInfo[];
  readonly cells: readonly Cell[];
}
