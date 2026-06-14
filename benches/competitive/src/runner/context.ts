// Capture machine + run context so results are reproducible and labelled
// "on this machine" (spec AC-10.11).

import { arch, availableParallelism, cpus, platform, release, totalmem } from "node:os";
import { FIXED_DATE } from "../workloads/data.ts";
import type { RunContext } from "./results.ts";

export interface CliOpts {
  readonly runs: number;
  readonly warmup: number;
  readonly referenceEngine: string;
}

function cpuModel(): string {
  const list = cpus();
  return list[0]?.model ?? "unknown";
}

/** Build the run context from the host + CLI options. */
export function buildContext(opts: CliOpts): RunContext {
  return {
    date: FIXED_DATE,
    os: `${platform()} ${release()}`,
    arch: arch(),
    cpuModel: cpuModel(),
    cores: availableParallelism(),
    totalMemBytes: totalmem(),
    nodeVersion: process.version,
    runs: opts.runs,
    warmup: opts.warmup,
    referenceEngine: opts.referenceEngine,
  };
}
