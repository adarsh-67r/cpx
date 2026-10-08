// Solution files on disk. Only the solution goes in the workspace; its tests,
// problem info, and statement are kept in the extension's storage.

import * as crypto from "crypto";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { Sample } from "./judge/parse";
import { ProblemRef, fileBaseName } from "./problemRef";
import { Language, SPECS } from "./languages";

/** Which problem a solution is for. */
export interface Stored {
  ref: ProblemRef;
  title: string;
}

/** The folder for solutions: the chosen one, else the first workspace folder, else home/cpx. */
export function solutionRoot(configured: string | undefined, firstWorkspace: string | undefined): string {
  if (configured && configured.trim()) return configured.trim();
  if (firstWorkspace) return firstWorkspace;
  return path.join(os.homedir(), "cpx");
}

export interface Meta {
  samples: string;
  problem: string;
  statement: string;
}

export interface Paths extends Meta {
  solution: string;
}

/** A solution's tests, problem info, and statement live in the extension's
 * storage (dataDir), named by a hash of the solution's path, so the
 * workspace holds only the solution file. */
export function metaFor(dataDir: string, solution: string): Meta {
  const key = path.resolve(solution);
  const id = crypto
    .createHash("sha1")
    .update(process.platform === "win32" ? key.toLowerCase() : key)
    .digest("hex")
    .slice(0, 16);
  const base = path.join(dataDir, id);
  return { samples: base + ".tests.json", problem: base + ".problem.json", statement: base + ".statement.txt" };
}

/** The older layout kept the same three files beside the solution. */
export function sideMetaFor(solution: string): Meta {
  const base = solution.slice(0, solution.length - path.extname(solution).length);
  return { samples: base + ".tests.json", problem: base + ".problem.json", statement: base + ".statement.txt" };
}

/** Where a new problem's files go. The solution sits in root (or root/<judge>
 * with perPlatform). If that name already belongs to another problem, a CSES
 * name gets its task id appended (WeirdAlgorithm-1068.cpp). */
export function pathsFor(root: string, ref: ProblemRef, title: string, language: Language, dataDir: string, perPlatform: boolean): Paths {
  const dir = perPlatform ? path.join(root, ref.platform) : root;
  const ext = SPECS[language].ext;
  let solution = path.join(dir, fileBaseName(ref, title) + ext);
  if (fs.existsSync(solution)) {
    const owner = readStored(metaFor(dataDir, solution).problem) ?? readStored(sideMetaFor(solution).problem);
    const ours = owner && owner.ref.platform === ref.platform && owner.ref.id === ref.id;
    if (!ours && ref.platform === "cses") {
      solution = path.join(dir, `${fileBaseName(ref, title)}-${ref.id}${ext}`);
    }
  }
  return { solution, ...metaFor(dataDir, solution) };
}

/** Writes the starting file unless one already exists: the user's template
 * file when one is set, else the built-in one. Returns true if written. */
export function scaffold(file: string, language: Language, templateFile?: string): boolean {
  if (fs.existsSync(file)) return false;
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const custom = templateFile && fs.existsSync(templateFile) ? fs.readFileSync(templateFile, "utf8") : undefined;
  fs.writeFileSync(file, custom ?? SPECS[language].template);
  return true;
}

/** The HTML statement sits next to the plain-text one. */
export function htmlStatementPath(statementFile: string): string {
  return statementFile.replace(/\.txt$/, ".html");
}

export function writeProblemFiles(p: Paths, stored: Stored, samples: Sample[], statement: string, statementHtml = ""): void {
  fs.mkdirSync(path.dirname(p.samples), { recursive: true });
  fs.writeFileSync(p.samples, JSON.stringify(samples, null, 2) + "\n");
  fs.writeFileSync(p.problem, JSON.stringify(stored, null, 2) + "\n");
  fs.writeFileSync(p.statement, statement + "\n");
  if (statementHtml) fs.writeFileSync(htmlStatementPath(p.statement), statementHtml + "\n");
}

export function readSamples(samplesFile: string): Sample[] {
  if (!fs.existsSync(samplesFile)) return [];
  return JSON.parse(fs.readFileSync(samplesFile, "utf8")) as Sample[];
}

export function readStored(problemFile: string): Stored | undefined {
  if (!fs.existsSync(problemFile)) return undefined;
  return JSON.parse(fs.readFileSync(problemFile, "utf8")) as Stored;
}

export function readStatement(statementFile: string): string | undefined {
  return fs.existsSync(statementFile) ? fs.readFileSync(statementFile, "utf8") : undefined;
}
