// Solution files on disk. Names and sample files match the Go core, so the
// terminal app and the extension share a workspace.

import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { Sample } from "./judge/parse";
import { ProblemRef, fileBaseName } from "./problemRef";
import { Language } from "./runner";

/** The starting text for a new solution. Same as the Go core's templates. */
export const templates: Record<Language, string> = {
  cpp: `#include <bits/stdc++.h>
using namespace std;

int main() {
    ios::sync_with_stdio(false);
    cin.tie(nullptr);

    return 0;
}
`,
  python: `import sys


def main():
    data = sys.stdin.read().split()


if __name__ == "__main__":
    main()
`,
};

export const extensionFor: Record<Language, string> = { cpp: ".cpp", python: ".py" };

/** Stored beside each solution: which problem it is. */
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

export interface Paths {
  solution: string;
  samples: string;
  problem: string;
  statement: string;
}

export function pathsFor(root: string, ref: ProblemRef, title: string, language: Language): Paths {
  const dir = path.join(root, ref.platform);
  const base = fileBaseName(ref, title);
  return {
    solution: path.join(dir, base + extensionFor[language]),
    samples: path.join(dir, base + ".tests.json"),
    problem: path.join(dir, base + ".problem.json"),
    statement: path.join(dir, base + ".statement.txt"),
  };
}

/** Writes the starting file unless one already exists. Returns true if written. */
export function scaffold(file: string, language: Language): boolean {
  if (fs.existsSync(file)) return false;
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, templates[language]);
  return true;
}

export function writeProblemFiles(p: Paths, stored: Stored, samples: Sample[], statement: string): void {
  fs.mkdirSync(path.dirname(p.samples), { recursive: true });
  fs.writeFileSync(p.samples, JSON.stringify(samples, null, 2) + "\n");
  fs.writeFileSync(p.problem, JSON.stringify(stored, null, 2) + "\n");
  fs.writeFileSync(p.statement, statement + "\n");
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
