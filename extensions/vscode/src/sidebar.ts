// The CPX sidebar: a verdict strip, Tests and Statement tabs, and Run all /
// Submit. The extension owns the state and sends it whole on every change;
// the page draws it and posts back what the user did. Kept free of VS Code
// imports so it can be tested.

import * as cheerio from "cheerio";
import katex from "katex";
import { Sample } from "./judge/parse";
import { CaseResult, Verdict, normalize } from "./runner";

export type ViewVerdict = Verdict | "CE";

export interface ViewTest {
  input: string;
  expected: string;
  result?: { verdict: ViewVerdict; timeMs: number; actual: string; error?: string; mismatch: number };
}

export interface ViewState {
  problem?: { title: string; url: string; label: string };
  statementHtml: string;
  tests: ViewTest[];
  compileError?: string;
  /** "all", a test index, or absent when idle. */
  running?: "all" | number;
}

export function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

/** The first line (0-based) where the output differs from the expected one, or -1. */
export function firstMismatch(expected: string, actual: string): number {
  const e = normalize(expected).split("\n");
  const a = normalize(actual).split("\n");
  for (let i = 0; i < Math.max(e.length, a.length); i++) {
    if ((e[i] ?? "") !== (a[i] ?? "")) return i;
  }
  return -1;
}

export function toViewTest(s: Sample, r?: CaseResult): ViewTest {
  return {
    input: s.Input,
    expected: s.Output,
    result: r && {
      verdict: r.verdict,
      timeMs: r.durationMs,
      actual: r.actual,
      error: r.error,
      mismatch: r.verdict === "WA" ? firstMismatch(s.Output, r.actual) : -1,
    },
  };
}

// The statement arrives as plain text, so section names are recognised by wording.
const HEADINGS = /^(input|output|constraints|examples?|notes?|interaction|scoring|problem statement|input format|output format|explanation)$/i;
const LIMITS = /^(time limit|memory limit)/i;

function tex(src: string, display: boolean): string {
  return katex.renderToString(src, { displayMode: display, throwOnError: false, output: "html" });
}

/** Paragraphs split on blank lines. $…$ and $$…$$ are typeset with KaTeX;
 * everything else is escaped text. */
export function statementHtml(text: string): string {
  return text
    .split(/\n{2,}/)
    .filter((p) => p.trim())
    .map((p) => {
      const src = p.trim();
      if (HEADINGS.test(src)) return `<h3>${escapeHtml(src)}</h3>`;
      if (LIMITS.test(src)) return `<p class="limits">${escapeHtml(src).replace(/\n/g, " ")}</p>`;
      let html = "";
      let last = 0;
      for (const m of src.matchAll(/\$\$([\s\S]+?)\$\$|\$([^$\n]+?)\$/g)) {
        html += escapeHtml(src.slice(last, m.index)).replace(/\n/g, "<br>");
        html += m[1] !== undefined ? `<div class="math">${tex(m[1], true)}</div>` : tex(m[2], false);
        last = m.index! + m[0].length;
      }
      html += escapeHtml(src.slice(last)).replace(/\n/g, "<br>");
      return `<p>${html}</p>`;
    })
    .join("\n");
}

/** Typesets the $…$ and $$…$$ math inside a saved (already cleaned) HTML
 * statement. Only text nodes are touched, and the text around math is
 * re-escaped, so markup can never be mistaken for math. */
export function statementHtmlWithMath(html: string): string {
  const $ = cheerio.load(html, null, false);
  const walk = (nodes: any[]) => {
    for (const n of nodes) {
      if (n.type === "text") {
        const src: string = n.data ?? "";
        if (!src.includes("$")) continue;
        let out = "";
        let last = 0;
        for (const m of src.matchAll(/\$\$([\s\S]+?)\$\$|\$([^$]+?)\$/g)) {
          out += escapeHtml(src.slice(last, m.index));
          out += m[1] !== undefined ? `<div class="math">${tex(m[1], true)}</div>` : tex(m[2], false);
          last = m.index! + m[0].length;
        }
        $(n).replaceWith(out + escapeHtml(src.slice(last)));
      } else if (n.type === "tag" && n.name !== "code") {
        walk([...(n.children ?? [])]);
      }
    }
  };
  walk([...$.root().contents().get()]);
  return $.html();
}

/** The statement with its sample tests as an Examples section, placed
 * before the Note section when there is one (as on the judge's page). */
export function withExamples(html: string, samples: Sample[]): string {
  if (!html.trim() || samples.length === 0) return html;
  const many = samples.length > 1;
  const examples =
    `<h3>${many ? "Examples" : "Example"}</h3>` +
    samples
      .map(
        (s, i) =>
          `<div class="example">${many ? `<div class="example-n">Example ${i + 1}</div>` : ""}<div class="example-io">` +
          `<div><span class="label">Input</span><pre>${escapeHtml(s.Input)}</pre></div>` +
          `<div><span class="label">Output</span><pre>${escapeHtml(s.Output)}</pre></div></div></div>`,
      )
      .join("");
  const note = html.search(/(<div>\s*)?<h3>\s*Notes?\s*<\/h3>/i);
  return note === -1 ? html + examples : html.slice(0, note) + examples + html.slice(note);
}

/** The page shell. Everything inside #app is drawn by the script from state messages. */
export function sidebarPage(cspSource: string, nonce: string, katexCss: string): string {
  return `<!doctype html><html><head><meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${cspSource} 'unsafe-inline'; font-src ${cspSource}; img-src ${cspSource} https: data:; script-src 'nonce-${nonce}';">
<meta name="viewport" content="width=device-width, initial-scale=1">
<link rel="stylesheet" href="${katexCss}">
<style>${CSS}</style></head>
<body><div id="app"></div>
<script nonce="${nonce}">${SCRIPT}</script>
</body></html>`;
}

const CSS = `
body.vscode-dark, body.vscode-high-contrast {
  --base: #1e1e2e; --mantle: #181825; --crust: #11111b; --surface: #313244; --overlay: #6c7086;
  --text: #cdd6f4; --sub: #a6adc8; --accent: #cba6f7;
  --ac: #a6e3a1; --wa: #f38ba8; --tle: #fab387; --re: #f9e2af; --ce: #89b4fa; --idle: #45475a;
}
body.vscode-light, body.vscode-high-contrast-light {
  --base: #eff1f5; --mantle: #e6e9ef; --crust: #dce0e8; --surface: #ccd0da; --overlay: #8c8fa1;
  --text: #4c4f69; --sub: #6c6f85; --accent: #8839ef;
  --ac: #40a02b; --wa: #d20f39; --tle: #fe640b; --re: #df8e1d; --ce: #1e66f5; --idle: #bcc0cc;
}
* { box-sizing: border-box; }
body { margin: 0; padding: 0; color: var(--text); background: var(--vscode-sideBar-background);
  font-family: var(--vscode-font-family); font-size: var(--vscode-font-size); line-height: 1.45; }
#app { display: flex; flex-direction: column; min-height: 100vh; }
button { font: inherit; color: inherit; cursor: pointer; border: 0; background: none; padding: 0; }
button:focus-visible, textarea:focus-visible, a:focus-visible { outline: 1px solid var(--accent); outline-offset: 1px; }
a { color: inherit; }

.head { padding: 12px 12px 10px; }
.title { font-size: 1.15em; font-weight: 600; margin: 0; text-decoration: none; display: block; }
.title:hover { color: var(--accent); }
.where { color: var(--sub); font-size: .9em; margin-top: 1px; display: flex; justify-content: space-between; }
.link { color: var(--sub); text-decoration: underline; text-underline-offset: 2px; }
.link:hover { color: var(--accent); }

.strip { display: flex; align-items: center; gap: 10px; padding: 0 12px 12px; }
.cells { display: flex; gap: 3px; flex-wrap: wrap; }
.cell { width: 18px; height: 18px; border-radius: 3px; background: var(--idle); font: 600 9px/18px var(--vscode-editor-font-family);
  text-align: center; color: var(--crust); }
.cell.running { animation: pulse 1s ease-in-out infinite; }
@keyframes pulse { 50% { opacity: .35; } }
@media (prefers-reduced-motion: reduce) { .cell.running { animation: none; opacity: .5; } }
.score { font-weight: 600; font-variant-numeric: tabular-nums; }
.score.all { color: var(--ac); }

.v-AC { background: var(--ac); } .v-WA { background: var(--wa); } .v-TLE { background: var(--tle); }
.v-RE { background: var(--re); } .v-CE { background: var(--ce); }
.t-AC { color: var(--ac); } .t-WA { color: var(--wa); } .t-TLE { color: var(--tle); }
.t-RE { color: var(--re); } .t-CE { color: var(--ce); }

.tabs { display: flex; gap: 16px; padding: 0 12px; border-bottom: 1px solid var(--surface); }
.tab { padding: 6px 0; color: var(--sub); border-bottom: 2px solid transparent; margin-bottom: -1px; }
.tab[aria-selected="true"] { color: var(--text); border-bottom-color: var(--accent); }

.body { flex: 1; padding: 10px 12px 12px; }
.test { border: 1px solid var(--surface); border-radius: 5px; margin-bottom: 10px; overflow: hidden; }
.test.pass { border-color: color-mix(in srgb, var(--ac) 45%, var(--surface)); }
.test.fail { border-color: color-mix(in srgb, var(--wa) 45%, var(--surface)); }
.test-head { display: flex; align-items: center; gap: 8px; padding: 4px 6px 4px 8px; background: var(--mantle); cursor: pointer; user-select: none; }
.test-name { font-weight: 600; }
.caret { color: var(--sub); width: 10px; display: inline-block; font-size: .8em; }
.chip { font: 700 10px/1 var(--vscode-editor-font-family); padding: 3px 5px; border-radius: 3px; border: 1px solid currentColor; }
.time { color: var(--sub); font-variant-numeric: tabular-nums; font-size: .92em; }
.test-tools { margin-left: auto; display: flex; gap: 2px; }
.icon { width: 22px; height: 22px; border-radius: 3px; color: var(--sub); }
.icon:hover { background: var(--surface); color: var(--text); }
.test-body { padding: 7px 8px 8px; display: flex; flex-direction: column; gap: 7px; }
.test.collapsed .test-body { display: none; }

.io { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 6px; }
.label { color: var(--sub); font-size: .85em; margin-bottom: 2px; display: block; }
textarea, pre { width: 100%; margin: 0; padding: 4px 6px; border-radius: 3px; border: 1px solid var(--surface);
  background: var(--vscode-input-background); color: var(--vscode-input-foreground);
  font: 12px/1.4 var(--vscode-editor-font-family); }
textarea { resize: none; display: block; overflow: auto; white-space: pre; }
textarea:focus { outline: none; border-color: var(--accent); }
pre { white-space: pre-wrap; word-break: break-word; max-height: 240px; overflow: auto; }
.line-bad { background: color-mix(in srgb, var(--wa) 25%, transparent); display: inline-block; width: 100%; }
.err { color: var(--wa); }
.none { color: var(--sub); font-style: italic; }

.add { width: 100%; padding: 6px; border: 1px dashed var(--surface); border-radius: 3px; color: var(--sub); }
.add:hover { color: var(--text); border-color: var(--overlay); }

.compile { border: 1px solid color-mix(in srgb, var(--ce) 45%, var(--surface)); border-radius: 5px; padding: 6px 8px; margin-bottom: 10px; }
.compile pre { margin-top: 5px; }

.statement p { margin: 0 0 .8em; max-width: 72ch; }
.statement h3 { font-size: 1em; font-weight: 600; color: var(--accent); margin: 1.1em 0 .35em; }
.statement .limits, .statement [data-limits] { color: var(--sub); margin-bottom: .3em; }
.statement .katex { font-size: 1.08em; }
.statement .math { margin: .5em 0; overflow-x: auto; overflow-y: hidden; }
.statement img { max-width: 100%; height: auto; display: block; margin: .6em auto; border-radius: 3px; background: #fff; }
.statement table { border-collapse: collapse; margin: .6em 0; display: block; overflow-x: auto; }
.statement td, .statement th { border: 1px solid var(--surface); padding: 3px 7px; }
.statement pre { margin: .4em 0; }
.statement ul, .statement ol { padding-left: 1.3em; margin: 0 0 .8em; }
.statement a { color: var(--accent); }
.statement hr { border: 0; border-top: 1px solid var(--surface); margin: 1em 0; }
.statement .example { margin: .4em 0 .9em; }
.statement .example-n { font-weight: 600; margin-bottom: 3px; }
.statement .example-io { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 6px; }
.statement .example pre { margin: 0; }

.bar { position: sticky; bottom: 0; display: flex; gap: 8px; padding: 10px 12px; background: var(--vscode-sideBar-background);
  border-top: 1px solid var(--surface); }
.bar button { flex: 1; padding: 6px 8px; border-radius: 3px; background: var(--vscode-button-secondaryBackground);
  color: var(--vscode-button-secondaryForeground); }
.bar .primary { background: var(--vscode-button-background); color: var(--vscode-button-foreground); }
.bar button:hover { filter: brightness(1.12); }
.bar button:disabled { opacity: .5; cursor: default; filter: none; }

.empty { padding: 16px 12px; color: var(--sub); }
.empty p { margin: 0 0 12px; }
.empty button { width: 100%; padding: 6px; border-radius: 3px; background: var(--vscode-button-background); color: var(--vscode-button-foreground); }
`;

// Plain browser JavaScript; runs inside the webview.
const SCRIPT = `
const vscode = acquireVsCodeApi();
const app = document.getElementById("app");
let state = null;
let tab = (vscode.getState() || {}).tab || "tests";
const collapsed = new Set((vscode.getState() || {}).collapsed || []);
const save = () => vscode.setState({ tab, collapsed: [...collapsed] });

const esc = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
const send = (type, extra) => vscode.postMessage(Object.assign({ type }, extra || {}));
const NAMES = { AC: "Accepted", WA: "Wrong answer", TLE: "Time limit exceeded", RE: "Runtime error", CE: "Compilation error" };

function outputHtml(r) {
  const lines = r.actual.replace(/\\r\\n/g, "\\n").replace(/\\n+$/, "").split("\\n");
  return lines.map((l, i) => i === r.mismatch ? '<span class="line-bad">' + esc(l || " ") + "</span>" : esc(l)).join("\\n");
}

function testHtml(t, i) {
  const r = t.result;
  const running = state.running === "all" || state.running === i;
  const v = r ? r.verdict : "";
  const open = !collapsed.has(i);
  let head = '<span class="caret">' + (open ? "&#9662;" : "&#9656;") + '</span><span class="test-name">Test ' + (i + 1) + "</span>";
  if (running) head += '<span class="time">running…</span>';
  else if (r) head += '<span class="chip t-' + v + '" title="' + NAMES[v] + '">' + v + "</span>" +
    (v === "CE" ? "" : '<span class="time">' + r.timeMs + " ms</span>");
  head += '<span class="test-tools">' +
    '<button class="icon" data-act="runOne" data-i="' + i + '" title="Run this test" aria-label="Run test ' + (i + 1) + '">&#9654;</button>' +
    '<button class="icon" data-act="remove" data-i="' + i + '" title="Delete this test" aria-label="Delete test ' + (i + 1) + '">&#10005;</button></span>';

  // Input and expected share a height: the taller of the two, within limits.
  const h = Math.min(12, Math.max(lines(t.input), lines(t.expected), 1));
  let out = "";
  if (r && v !== "CE") {
    out = '<div><span class="label">Output</span>' + (r.actual.trim() ? "<pre>" + outputHtml(r) + "</pre>" : '<pre class="none">no output</pre>') + "</div>";
    if (r.error && v !== "TLE") out += '<div><span class="label">Error</span><pre class="err">' + esc(r.error) + "</pre></div>";
  }
  const state_ = r ? (v === "AC" ? " pass" : " fail") : "";
  return '<section class="test' + state_ + (open ? "" : " collapsed") + '">' +
    '<div class="test-head" data-act="toggle" data-i="' + i + '" aria-expanded="' + open + '">' + head + "</div>" +
    '<div class="test-body"><div class="io">' +
      '<div><label class="label" for="in' + i + '">Input</label><textarea id="in' + i + '" data-i="' + i + '" data-k="input" spellcheck="false" rows="' + h + '">' + esc(t.input) + "</textarea></div>" +
      '<div><label class="label" for="ex' + i + '">Expected</label><textarea id="ex' + i + '" data-i="' + i + '" data-k="expected" spellcheck="false" rows="' + h + '">' + esc(t.expected) + "</textarea></div>" +
    "</div>" + out + "</div></section>";
}

function lines(s) { return String(s).replace(/\\n+$/, "").split("\\n").length; }

function render() {
  if (!state || !state.problem) {
    app.innerHTML = '<div class="empty"><p>Open a problem to get its tests and statement here. Solutions CPX created show up when you switch to them.</p>' +
      '<button data-act="openProblem">Open problem from URL</button></div>';
    return;
  }
  const p = state.problem;
  const done = state.tests.filter((t) => t.result);
  const passed = done.filter((t) => t.result.verdict === "AC").length;
  const cells = state.tests.map((t, i) => {
    const running = state.running === "all" || state.running === i;
    const v = t.result ? t.result.verdict : "";
    return '<span class="cell ' + (v ? "v-" + v : "") + (running ? " running" : "") + '" title="Test ' + (i + 1) + (v ? ": " + NAMES[v] : "") + '">' + (i + 1) + "</span>";
  }).join("");
  const score = done.length ? '<span class="score' + (passed === state.tests.length ? " all" : "") + '">' + passed + "/" + state.tests.length + " passed</span>" : "";

  let body;
  if (tab === "statement") {
    body = '<div class="statement">' + (state.statementHtml || '<p class="where">No statement saved for this problem.</p>') + "</div>";
  } else {
    body = (state.compileError ? '<div class="compile"><span class="verdict t-CE">CE</span> <span class="time">Compilation error</span><pre class="err">' + esc(state.compileError) + "</pre></div>" : "") +
      state.tests.map(testHtml).join("") +
      '<button class="add" data-act="add">Add test</button>';
  }
  const busy = state.running !== undefined && state.running !== null;
  app.innerHTML =
    '<div class="head"><a class="title" href="' + esc(p.url) + '" title="Open on the judge">' + esc(p.title) + '</a><div class="where">' + esc(p.label) + ' <button class="link" data-act="editorial">Find editorial</button></div></div>' +
    '<div class="strip"><div class="cells">' + cells + "</div>" + score + "</div>" +
    '<div class="tabs" role="tablist">' +
      '<button class="tab" role="tab" data-tab="tests" aria-selected="' + (tab === "tests") + '">Tests</button>' +
      '<button class="tab" role="tab" data-tab="statement" aria-selected="' + (tab === "statement") + '">Statement</button></div>' +
    '<div class="body">' + body + "</div>" +
    '<div class="bar"><button class="primary" data-act="run"' + (busy ? " disabled" : "") + ">" + (busy ? "Running…" : "Run all") + "</button>" +
    '<button data-act="submit">Submit</button></div>';
}

function tests() {
  return state.tests.map((t) => ({ input: t.input, expected: t.expected }));
}

app.addEventListener("click", (e) => {
  const el = e.target.closest("[data-act], [data-tab]");
  if (!el) return;
  if (el.dataset.tab) { tab = el.dataset.tab; save(); render(); return; }
  const act = el.dataset.act, i = Number(el.dataset.i);
  if (act === "toggle") {
    if (e.target.closest(".test-tools")) return;
    collapsed.has(i) ? collapsed.delete(i) : collapsed.add(i);
    save();
    render();
    return;
  }
  if (act === "add") { send("save", { tests: tests().concat([{ input: "", expected: "" }]) }); return; }
  if (act === "remove") { send("save", { tests: tests().filter((_, j) => j !== i) }); return; }
  if (act === "runOne") { send("run", { index: i }); return; }
  send(act);
});

// Edits are kept in the page while typing and saved when the field loses focus.
app.addEventListener("input", (e) => {
  const el = e.target;
  if (el.tagName !== "TEXTAREA") return;
  state.tests[Number(el.dataset.i)][el.dataset.k] = el.value;
});
app.addEventListener("change", (e) => {
  if (e.target.tagName === "TEXTAREA") send("save", { tests: tests() });
});

window.addEventListener("message", (e) => {
  if (e.data.type !== "state") return;
  // Do not redraw under the cursor: it would drop the edit in progress.
  const typing = document.activeElement && document.activeElement.tagName === "TEXTAREA";
  state = e.data.state;
  if (!typing) render();
});
send("ready");
`;
