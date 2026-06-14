// Summary statistics over a sample of measurements.

export interface Summary {
  readonly n: number;
  readonly min: number;
  readonly p50: number;
  readonly p95: number;
  readonly p99: number;
  readonly max: number;
  readonly mean: number;
}

/** Linear-interpolation percentile over a pre-sorted ascending array. */
function percentile(sorted: readonly number[], q: number): number {
  const n = sorted.length;
  if (n < 2) return sorted[0] ?? Number.NaN;
  const pos = q * (n - 1);
  const lo = Math.floor(pos);
  const lov = sorted[lo] ?? 0;
  const hiv = sorted[Math.ceil(pos)] ?? 0;
  return lov + (hiv - lov) * (pos - lo);
}

/** Summarise a non-empty sample; returns NaN-filled summary for empty input. */
export function summarize(samples: readonly number[]): Summary {
  const sorted = [...samples].sort((a, b) => a - b);
  const n = sorted.length;
  const sum = sorted.reduce((acc, v) => acc + v, 0);
  return {
    n,
    min: sorted[0] ?? Number.NaN,
    p50: percentile(sorted, 0.5),
    p95: percentile(sorted, 0.95),
    p99: percentile(sorted, 0.99),
    max: sorted[n - 1] ?? Number.NaN,
    mean: n > 0 ? sum / n : Number.NaN,
  };
}
