// Supported languages: file extension, how to build and run, and the starting
// template. Commands are argument lists with {source}, {exe}, {out}, and {dir}
// filled in by the runner; programs start directly, with no shell.

export const LANGUAGES = ["cpp", "c", "python", "pypy", "java", "kotlin", "rust", "go", "javascript", "ruby", "haskell"] as const;
export type Language = (typeof LANGUAGES)[number];

export interface Commands {
  compile?: string[];
  run: string[];
}

interface Spec extends Commands {
  name: string;
  ext: string;
  template: string;
}

const CPP_TEMPLATE = `#include <bits/stdc++.h>
using namespace std;

int main() {
    ios::sync_with_stdio(false);
    cin.tie(nullptr);

    return 0;
}
`;

const PY_TEMPLATE = `import sys


def main():
    data = sys.stdin.read().split()


if __name__ == "__main__":
    main()
`;

export const SPECS: Record<Language, Spec> = {
  cpp: { name: "C++", ext: ".cpp", compile: ["g++", "-std=c++17", "-O2", "-o", "{exe}", "{source}"], run: ["{exe}"], template: CPP_TEMPLATE },
  c: {
    name: "C",
    ext: ".c",
    compile: ["gcc", "-std=c11", "-O2", "-o", "{exe}", "{source}", "-lm"],
    run: ["{exe}"],
    template: `#include <stdio.h>\n\nint main(void) {\n\n    return 0;\n}\n`,
  },
  python: { name: "Python", ext: ".py", run: ["python", "{source}"], template: PY_TEMPLATE },
  pypy: { name: "PyPy", ext: ".py", run: ["pypy3", "{source}"], template: PY_TEMPLATE },
  // Java 11+ runs a single source file directly; the class need not match the file name.
  java: {
    name: "Java",
    ext: ".java",
    run: ["java", "{source}"],
    template: `import java.util.*;\nimport java.io.*;\n\npublic class Main {\n    public static void main(String[] args) throws IOException {\n        BufferedReader in = new BufferedReader(new InputStreamReader(System.in));\n\n    }\n}\n`,
  },
  kotlin: {
    name: "Kotlin",
    ext: ".kt",
    compile: ["kotlinc", "{source}", "-include-runtime", "-d", "{out}.jar"],
    run: ["java", "-jar", "{out}.jar"],
    template: `fun main() {\n    val n = readln().trim()\n\n}\n`,
  },
  rust: {
    name: "Rust",
    ext: ".rs",
    compile: ["rustc", "-O", "--edition", "2021", "-o", "{exe}", "{source}"],
    run: ["{exe}"],
    template: `use std::io::{self, Read};\n\nfn main() {\n    let mut input = String::new();\n    io::stdin().read_to_string(&mut input).unwrap();\n    let mut it = input.split_ascii_whitespace();\n\n}\n`,
  },
  go: {
    name: "Go",
    ext: ".go",
    compile: ["go", "build", "-o", "{exe}", "{source}"],
    run: ["{exe}"],
    template: `package main\n\nimport (\n\t"bufio"\n\t"fmt"\n\t"os"\n)\n\nfunc main() {\n\tin := bufio.NewReader(os.Stdin)\n\tout := bufio.NewWriter(os.Stdout)\n\tdefer out.Flush()\n\t_, _ = in, fmt.Fprint\n}\n`,
  },
  javascript: {
    name: "JavaScript",
    ext: ".js",
    run: ["node", "{source}"],
    template: `const data = require("fs").readFileSync(0, "utf8").trim().split(/\\s+/);\n\n`,
  },
  ruby: { name: "Ruby", ext: ".rb", run: ["ruby", "{source}"], template: `data = STDIN.read.split\n\n` },
  haskell: {
    name: "Haskell",
    ext: ".hs",
    compile: ["ghc", "-O2", "-outputdir", "{dir}", "-o", "{exe}", "{source}"],
    run: ["{exe}"],
    template: `main :: IO ()\nmain = do\n  input <- getContents\n  return ()\n`,
  },
};

/** The language for a file extension. .py is PyPy when that is the default language. */
export function languageForExt(ext: string, defaultLanguage: string): Language | undefined {
  if (ext === ".py") return defaultLanguage === "pypy" ? "pypy" : "python";
  return LANGUAGES.find((l) => SPECS[l].ext === ext);
}

/** Splits a command line into arguments, keeping "quoted parts" together. */
export function splitCommand(line: string): string[] {
  const out: string[] = [];
  for (const m of line.matchAll(/"([^"]*)"|'([^']*)'|(\S+)/g)) out.push(m[1] ?? m[2] ?? m[3]);
  return out;
}

/** Built-in commands, with any override from settings ({ compile, run } as command lines). */
export function commandsFor(language: Language, override?: { compile?: string; run?: string }): Commands {
  const spec = SPECS[language];
  if (!override) return { compile: spec.compile, run: spec.run };
  return {
    compile: override.compile !== undefined ? (override.compile.trim() ? splitCommand(override.compile) : undefined) : spec.compile,
    run: override.run?.trim() ? splitCommand(override.run) : spec.run,
  };
}
