// Adapter helpers: optional dynamic import, binary detection, high-res timing.

import { execFile } from "node:child_process";
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { promisify } from "node:util";
import type { Availability, Footprint } from "../types.ts";

const exec = promisify(execFile);
const require = createRequire(import.meta.url);

/** Monotonic wall-clock in milliseconds. */
export function nowMs(): number {
  return Number(process.hrtime.bigint()) / 1e6;
}

/** Try to import an optional package; return null if it is not installed. */
export async function tryImport<T>(spec: string): Promise<T | null> {
  try {
    return (await import(spec)) as T;
  } catch {
    return null;
  }
}

/** Detect a binary on PATH and capture its version line. */
export async function detectBinary(
  bin: string,
  versionArgs: readonly string[],
): Promise<Availability> {
  try {
    const { stdout, stderr } = await exec(bin, [...versionArgs], { timeout: 8000 });
    const out = (stdout || stderr).trim().split("\n")[0] ?? bin;
    return { available: true, version: out, reason: null };
  } catch {
    return { available: false, version: null, reason: `${bin} not found on PATH` };
  }
}

function versionFrom(json: unknown): string | null {
  const v = (json as { version?: unknown } | null)?.version;
  return typeof v === "string" ? v : null;
}

/** Read a package's version from its installed package.json, or null. */
export function pkgVersion(name: string): string | null {
  try {
    const path = require.resolve(`${name}/package.json`);
    return versionFrom(JSON.parse(readFileSync(path, "utf8")));
  } catch {
    return null;
  }
}

/** A package-based availability check. */
export function detectPackage(name: string): Promise<Availability> {
  const version = pkgVersion(name);
  const ok: Availability = { available: true, version: version ?? "", reason: null };
  const no: Availability = {
    available: false,
    version: null,
    reason: `npm package ${name} not installed`,
  };
  return Promise.resolve(version ? ok : no);
}

/** Default footprint for a pure-JS dependency that ships no browser. */
export function jsFootprint(notes: string): Footprint {
  return { installedBytes: null, shipsBrowser: false, notes };
}
