import { test } from "node:test";
import * as assert from "node:assert/strict";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { execFileSync } from "child_process";
import { parseAtCoder, parseCodeforces, parseCSES } from "../src/judge/parse";
import { fileBaseName, parseProblemUrl, pascal, submitUrl } from "../src/problemRef";
import { escapeHtml, firstMismatch, sidebarPage, statementHtml, toViewTest, withExamples } from "../src/sidebar";
import { normalize, runSamples, RunSpec } from "../src/runner";
import { commandsFor, languageForExt, splitCommand } from "../src/languages";
import { metaFor, pathsFor, sideMetaFor, writeProblemFiles } from "../src/workspace";
import { statementHtmlWithMath } from "../src/sidebar";
import * as cheerio from "cheerio";
import { cleanStatement } from "../src/judge/statementHtml";

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

test("escaping covers html specials", () => {
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
  const spec: RunSpec = { commands: commandsFor("python"), source: src, buildDir: dir, timeLimitMs: 5000 };
  const results = await runSamples(spec, [
    { Input: "1 2", Output: "3" },
    { Input: "2 2", Output: "5" },
  ]);
  assert.equal(results[0].passed, true);
  assert.equal(results[0].verdict, "AC");
  assert.equal(results[1].passed, false);
  assert.equal(results[1].verdict, "WA");
  assert.equal(results[1].actual.trim(), "4");
});

test("statement typesets math and escapes text", () => {
  const html = statementHtml("Find $a+b$ where <b> is big.\n\n$$x^2$$");
  assert.match(html, /class="katex"/);
  assert.match(html, /&lt;b&gt;/);
  assert.match(html, /<div class="math"><span class="katex-display">/);
  assert.doesNotMatch(html, /\$/);
});

test("wrong answers point at the first differing line", () => {
  assert.equal(firstMismatch("1\n2\n3", "1\n2\n3  \n"), -1);
  assert.equal(firstMismatch("1\n2\n3", "1\n5\n3"), 1);
  assert.equal(firstMismatch("1\n2", "1"), 1);
  const t = toViewTest({ Input: "x", Output: "YES" }, { index: 1, verdict: "WA", passed: false, input: "x", expected: "YES", actual: "NO", durationMs: 4 });
  assert.equal(t.result?.mismatch, 0);
  assert.equal(t.result?.timeMs, 4);
});

test("sidebar page script parses and is locked to its nonce", () => {
  const page = sidebarPage("vscode-resource:", "abc123", "katex.css");
  assert.match(page, /script-src 'nonce-abc123'/);
  const script = page.match(/<script nonce="abc123">([\s\S]*)<\/script>/)?.[1] ?? "";
  assert.ok(script.length > 100);
  assert.doesNotThrow(() => new Function(script));
});

const has = (cmd: string) => {
  try {
    execFileSync(cmd, ["--version"], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
};

test("languages come from file extensions, and settings can override commands", () => {
  assert.equal(languageForExt(".rs", "cpp"), "rust");
  assert.equal(languageForExt(".py", "cpp"), "python");
  assert.equal(languageForExt(".py", "pypy"), "pypy");
  assert.equal(languageForExt(".txt", "cpp"), undefined);
  assert.deepEqual(splitCommand(`g++ -o "{exe}" 'my file.cpp'`), ["g++", "-o", "{exe}", "my file.cpp"]);
  const c = commandsFor("cpp", { compile: "clang++ -O2 -o {exe} {source}" });
  assert.deepEqual(c.compile, ["clang++", "-O2", "-o", "{exe}", "{source}"]);
  assert.deepEqual(c.run, ["{exe}"]);
  assert.equal(commandsFor("cpp", { compile: "" }).compile, undefined);
});

test("C++ builds and runs; a compile error throws", async (t) => {
  if (!has("g++")) return t.skip("g++ not on PATH");
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cpx-cpp-"));
  const src = path.join(dir, "a b.cpp");
  fs.writeFileSync(src, "#include <iostream>\nint main(){int a,b;std::cin>>a>>b;std::cout<<a+b;}\n");
  const spec: RunSpec = { commands: commandsFor("cpp"), source: src, buildDir: dir, timeLimitMs: 5000 };
  const [r] = await runSamples(spec, [{ Input: "2 3", Output: "5" }]);
  assert.equal(r.verdict, "AC");
  fs.writeFileSync(src, "int main( {");
  await assert.rejects(runSamples(spec, [{ Input: "", Output: "" }]), /Compilation failed/);
});

test("a slow program gets TLE", async (t) => {
  if (!has("python")) return t.skip("python not on PATH");
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cpx-tle-"));
  const src = path.join(dir, "slow.py");
  fs.writeFileSync(src, "import time\ntime.sleep(5)\n");
  const started = Date.now();
  const [r] = await runSamples({ commands: commandsFor("python"), source: src, buildDir: dir, timeLimitMs: 300 }, [{ Input: "", Output: "" }]);
  assert.equal(r.verdict, "TLE");
  assert.ok(Date.now() - started < 4000);
});

test("only the solution goes in the workspace; names clash safely", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "cpx-ws-"));
  const data = path.join(root, ".data");
  const cf = parseProblemUrl("https://codeforces.com/problemset/problem/4/A")!;
  const p = pathsFor(root, cf, "Watermelon", "cpp", data, false);
  assert.equal(p.solution, path.join(root, "4A.cpp"));
  assert.ok(p.samples.startsWith(data), "extras live in the extension's storage");
  assert.equal(pathsFor(root, cf, "Watermelon", "cpp", data, true).solution, path.join(root, "codeforces", "4A.cpp"));
  assert.deepEqual(metaFor(data, p.solution), metaFor(data, path.join(root, ".", "4A.cpp")), "same file, same key");

  // A CSES name already used by another task gets the task id appended.
  const a = parseProblemUrl("https://cses.fi/problemset/task/1068")!;
  const first = pathsFor(root, a, "Weird Algorithm", "cpp", data, false);
  fs.writeFileSync(first.solution, "");
  writeProblemFiles(first, { ref: a, title: "Weird Algorithm" }, [], "");
  assert.equal(pathsFor(root, a, "Weird Algorithm", "cpp", data, false).solution, first.solution, "reopening keeps the file");
  const b = { ...a, id: "9999", url: "https://cses.fi/problemset/task/9999" };
  assert.equal(pathsFor(root, b, "Weird Algorithm", "cpp", data, false).solution, path.join(root, "WeirdAlgorithm-9999.cpp"));

  assert.equal(sideMetaFor(path.join(root, "4A.cpp")).samples, path.join(root, "4A.tests.json"));
});

test("statement HTML keeps structure and images but nothing unsafe", () => {
  const $ = cheerio.load(`<div id="s">
    <p onclick="x()">Given <span class="math math-inline">n</span> points.</p>
    <img src="/images/fig1.png" onerror="x()"><img src="javascript:x()">
    <a href="javascript:alert(1)">bad</a><a href="/blog/1">ok</a>
    <script>alert(1)</script><iframe src="https://evil"></iframe>
    <table><tr><td colspan="2" style="color:red">cell</td></tr></table>
    <ul><li>one</li></ul><font color="red">kept text</font>
  </div>`);
  const html = cleanStatement($, $("#s"), "https://cses.fi/problemset/task/1068");
  assert.match(html, /<img src="https:\/\/cses.fi\/images\/fig1.png">/);
  assert.doesNotMatch(html, /javascript:|onclick|onerror|<script|<iframe|style=|<font/);
  assert.match(html, /<a href="https:\/\/cses.fi\/blog\/1">ok<\/a>/);
  assert.match(html, /<td colspan="2">cell<\/td>/);
  assert.match(html, /<li>one<\/li>/);
  assert.match(html, /kept text/);
  assert.match(html, /Given \$n\$ points/);
  const shown = statementHtmlWithMath(html);
  assert.match(shown, /class="katex"/);
  assert.doesNotMatch(shown, /\$n\$/);
});

test("judge pages give HTML statements without the samples", () => {
  for (const [p, f] of [[parseCodeforces, "cf_4A.html"], [parseCSES, "cses_1068.html"], [parseAtCoder, "atcoder_abc001_1.html"]] as const) {
    const s = p(fixture(f)).statementHtml;
    assert.ok(s.length > 100, f);
    assert.doesNotMatch(s, /<script|Sample Input|入力例 1|id="example"/, f);
  }
});

test("Codeforces header becomes one limits line", () => {
  const html = parseCodeforces(fixture("cf_4A.html")).statementHtml;
  assert.match(html, /<p data-limits="">1 second · 64 megabytes<\/p>/);
  assert.doesNotMatch(html, /time limit per test|standard input|A\. Watermelon/);
});

test("examples go into the statement before the note", () => {
  const html = withExamples("<p>Story.</p><div><h3>Note</h3><p>Hint.</p></div>", [{ Input: "1 <2>", Output: "3" }, { Input: "4", Output: "5" }]);
  assert.ok(html.indexOf("Examples") < html.indexOf("Note") && html.indexOf("Story") < html.indexOf("Examples"));
  assert.match(html, /Example 2/);
  assert.match(html, /1 &lt;2&gt;/);
  assert.match(withExamples("<p>CSES text</p>", [{ Input: "3", Output: "3 10" }]), /CSES text<\/p><h3>Example<\/h3>/);
  assert.equal(withExamples("", [{ Input: "1", Output: "1" }]), "");
});

test("headings are one level, separators collapse, format blocks get math", () => {
  const $ = cheerio.load(`<div id="s"><h1>Input</h1><p>x</p><hr><hr><hr><pre>$N$ $A$</pre><hr></div>`);
  const html = cleanStatement($, $("#s"), "https://cses.fi/");
  assert.match(html, /<h3>Input<\/h3>/);
  assert.equal((html.match(/<hr>/g) ?? []).length, 1);
  assert.doesNotMatch(html, /<hr>\s*$/);
  assert.match(statementHtmlWithMath(html), /<pre><span class="katex">/);
  assert.match(parseCSES(fixture("cses_1068.html")).statementHtml, /<h3>Input<\/h3>/);
});
