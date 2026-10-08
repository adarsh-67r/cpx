import { test } from "node:test";
import * as assert from "node:assert/strict";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { execFileSync } from "child_process";
import { parseAtCoder, parseCodeforces, parseCSES } from "../src/judge/parse";
import { fileBaseName, parseProblemUrl, pascal, submitUrl } from "../src/problemRef";
import { renderResults, escapeHtml } from "../src/results";
import { normalize, runSamples, RunSpec } from "../src/runner";

const fixture = (name: string) => fs.readFileSync(path.join(__dirname, "..", "..", "test", "fixtures", name), "utf8");

test("problem URLs map to platform, id, and page", () => {
  assert.deepEqual(parseProblemUrl("https://codeforces.com/problemset/problem/4/A"), {
    platform: "codeforces",
    id: "4A",
    url: "https://codeforces.com/problemset/problem/4/A",
  });
  assert.equal(parseProblemUrl("https://cses.fi/problemset/task/1068")?.id, "1068");
  assert.equal(parseProblemUrl("https://atcoder.jp/contests/abc001/tasks/abc001_1")?.id, "abc001_1");
  assert.equal(parseProblemUrl("https://example.com/whatever"), undefined);
});

test("file names match the Go core", () => {
  assert.equal(fileBaseName({ platform: "codeforces", id: "4A", url: "" }, "Watermelon"), "4A");
  assert.equal(fileBaseName({ platform: "cses", id: "1068", url: "" }, "Weird Algorithm"), "WeirdAlgorithm");
  assert.equal(fileBaseName({ platform: "atcoder", id: "ABC001_1", url: "" }, "x"), "abc001_1");
  assert.equal(pascal("Two Sets (easy)"), "TwoSetsEasy");
});

test("submit pages", () => {
  assert.equal(submitUrl({ platform: "codeforces", id: "1095F", url: "" }), "https://codeforces.com/problemset/submit/1095/F");
  assert.equal(submitUrl({ platform: "cses", id: "1068", url: "" }), "https://cses.fi/problemset/submit/1068");
  assert.equal(
    submitUrl({ platform: "atcoder", id: "abc001_1", url: "" }),
    "https://atcoder.jp/contests/abc001/submit?taskScreenName=abc001_1",
  );
});

test("Codeforces page gives title and samples", () => {
  const p = parseCodeforces(fixture("cf_4A.html"));
  assert.equal(p.title, "Watermelon");
  assert.deepEqual(p.samples, [{ Input: "8", Output: "YES" }]);
  assert.match(p.statement, /Pete and his friend Billy/);
});

test("CSES page gives samples and statement without the example", () => {
  const p = parseCSES(fixture("cses_1068.html"));
  assert.deepEqual(p.samples, [{ Input: "3", Output: "3 10 5 16 8 4 2 1" }]);
  assert.match(p.statement, /the sequence for/);
  assert.doesNotMatch(p.statement, /Example/);
});

test("AtCoder page gives paired samples", () => {
  const p = parseAtCoder(fixture("atcoder_abc001_1.html"));
  assert.equal(p.samples.length, 3);
  assert.deepEqual(p.samples[0], { Input: "15\n10", Output: "5" });
});

test("output comparison ignores trailing spaces and blank lines", () => {
  assert.equal(normalize("3 \n4\n\n"), normalize("3\n4"));
  assert.notEqual(normalize("3\n4"), normalize("3\n5"));
});

test("results page escapes program output", () => {
  const html = renderResults("A", [
    { index: 1, passed: false, input: "<x>", expected: "1", actual: "2", durationMs: 3 },
  ]);
  assert.match(html, /&lt;x&gt;/);
  assert.doesNotMatch(html, /<x>/);
  assert.equal(escapeHtml(`a&"<`), "a&amp;&quot;&lt;");
});

test("runner passes and fails samples with a Python solution", async (t) => {
  try {
    execFileSync("python", ["--version"], { stdio: "ignore" });
  } catch {
    t.skip("python not on PATH");
    return;
  }
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cpx-test-"));
  const src = path.join(dir, "sum.py");
  fs.writeFileSync(src, "a, b = map(int, input().split())\nprint(a + b)\n");
  const spec: RunSpec = {
    language: "python",
    source: src,
    buildDir: dir,
    cppCompiler: "g++",
    pythonPath: "python",
    timeLimitMs: 5000,
  };
  const results = await runSamples(spec, [
    { Input: "1 2", Output: "3" },
    { Input: "2 2", Output: "5" },
  ]);
  assert.equal(results[0].passed, true);
  assert.equal(results[1].passed, false);
  assert.equal(results[1].actual.trim(), "4");
});
