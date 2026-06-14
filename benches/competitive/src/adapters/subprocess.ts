// Shared helpers for subprocess-based engines (Typst, wkhtmltopdf, WeasyPrint).
// Each writes the workload to a temp input, invokes a binary, reads the PDF back.

import { execFile } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

const exec = promisify(execFile);

/** Run a CLI engine over a written input file and read the produced PDF. */
export async function runCliEngine(opts: {
  bin: string;
  inputName: string;
  inputData: string;
  outputName: string;
  argv: (input: string, output: string) => string[];
}): Promise<Uint8Array> {
  const dir = await mkdtemp(join(tmpdir(), "tpdf-bench-"));
  try {
    const input = join(dir, opts.inputName);
    const output = join(dir, opts.outputName);
    await writeFile(input, opts.inputData);
    await exec(opts.bin, opts.argv(input, output), { timeout: 120000 });
    const buf = await readFile(output);
    return new Uint8Array(buf);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
}
