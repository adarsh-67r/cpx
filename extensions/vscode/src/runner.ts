// Compiles and runs a solution against sample tests. Programs start directly,
// with no shell in between, so paths with spaces need no quoting.

import { spawn } from "child_process";
import * as path from "path";
import { Sample } from "./judge/parse";
import { Commands } from "./languages";

export interface RunSpec {
  commands: Commands;
  source: string; // absolute path to the solution file
  buildDir: string; // scratch folder for compiled programs
  timeLimitMs: number;
}

/** CE (compile error) applies to the whole run, so it is not a per-case verdict. */
export type Verdict = "AC" | "WA" | "TLE" | "RE";

export interface CaseResult {
  index: number;
  verdict: Verdict;
  passed: boolean;
  input: string;
  expected: string;
  actual: string;
  durationMs: number;
  error?: string;
}

/** Trailing spaces on each line and trailing blank lines do not count. */
export function normalize(s: string): string {
  return s
    .replace(/\r\n/g, "\n")
    .split("\n")
    .map((l) => l.replace(/[ \t]+$/, ""))
    .join("\n")
    .replace(/\n+$/, "");
}

interface Done {
  code: number | null;
  stdout: string;
  stderr: string;
  timedOut: boolean;
  durationMs: number;
}

function exec(cmd: string, args: string[], cwd: string, stdin: string | undefined, timeoutMs: number): Promise<Done> {
  return new Promise((resolve, reject) => {
    const started = Date.now();
    const child = spawn(cmd, args, { cwd, stdio: ["pipe", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    let timedOut = false;
    child.stdout.on("data", (d) => (stdout += d.toString()));
    child.stderr.on("data", (d) => (stderr += d.toString()));
    child.on("error", (e) => reject(e));

    const timer = setTimeout(() => {
      timedOut = true;
      child.kill();
    }, timeoutMs);

    child.on("close", (code) => {
      clearTimeout(timer);
      resolve({ code, stdout, stderr, timedOut, durationMs: Date.now() - started });
    });
    if (stdin !== undefined) child.stdin.write(stdin);
    child.stdin.end();
  });
}

/** Fills {source}, {exe}, {out}, and {dir} into a command. */
export function expand(args: string[], spec: RunSpec): string[] {
  const out = path.join(spec.buildDir, path.basename(spec.source, path.extname(spec.source)));
  const exe = out + (process.platform === "win32" ? ".exe" : "");
  return args.map((a) =>
    a.replace(/{source}/g, () => spec.source).replace(/{exe}/g, () => exe).replace(/{out}/g, () => out).replace(/{dir}/g, () => spec.buildDir),
  );
}

/** Runs the compile step, if the language has one. Throws on failure. */
async function build(spec: RunSpec): Promise<string[]> {
  if (spec.commands.compile) {
    const [cmd, ...args] = expand(spec.commands.compile, spec);
    let done: Done;
    try {
      done = await exec(cmd, args, spec.buildDir, undefined, 120_000);
    } catch (e) {
      throw new Error(`Compilation failed:\ncould not start ${cmd}: ${(e as Error).message}`);
    }
    if (done.code !== 0) {
      throw new Error("Compilation failed:\n" + (done.stderr || done.stdout).trim());
    }
  }
  return expand(spec.commands.run, spec);
}

/** Runs every sample. A compile failure throws; a bad sample is reported in its result. */
export async function runSamples(spec: RunSpec, samples: Sample[]): Promise<CaseResult[]> {
  const [cmd, ...args] = await build(spec);
  const results: CaseResult[] = [];
  for (let i = 0; i < samples.length; i++) {
    const s = samples[i];
    const base = { index: i + 1, input: s.Input, expected: s.Output };
    let done: Done;
    try {
      done = await exec(cmd, args, spec.buildDir, s.Input, spec.timeLimitMs);
    } catch (e) {
      results.push({ ...base, verdict: "RE", passed: false, actual: "", durationMs: 0, error: String(e) });
      continue;
    }
    if (done.timedOut) {
      results.push({ ...base, verdict: "TLE", passed: false, actual: done.stdout, durationMs: done.durationMs, error: "time limit exceeded" });
      continue;
    }
    if (done.code !== 0) {
      results.push({ ...base, verdict: "RE", passed: false, actual: done.stdout, durationMs: done.durationMs, error: done.stderr.trim() || `exit code ${done.code}` });
      continue;
    }
    const passed = normalize(done.stdout) === normalize(s.Output);
    results.push({
      ...base,
      verdict: passed ? "AC" : "WA",
      passed,
      actual: done.stdout,
      durationMs: done.durationMs,
    });
  }
  return results;
}
