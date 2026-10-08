// CPX for VS Code: open a problem from its URL, run the samples on the
// solution, copy the code to submit it by hand, and read the statement.

import * as fs from "fs";
import * as https from "https";
import * as path from "path";
import * as vscode from "vscode";
import { parseAtCoder, parseCodeforces, parseCSES, ParsedProblem, Sample } from "./judge/parse";
import { ProblemRef, parseProblemUrl, submitUrl } from "./problemRef";
import { Language, SPECS, commandsFor, languageForExt } from "./languages";
import { CaseResult, RunSpec, runSamples } from "./runner";
import { ViewState, sidebarPage, statementHtml, statementHtmlWithMath, toViewTest, withExamples } from "./sidebar";
import {
  htmlStatementPath,
  metaFor,
  pathsFor,
  readSamples,
  readStatement,
  readStored,
  scaffold,
  sideMetaFor,
  solutionRoot,
  writeProblemFiles,
} from "./workspace";

const USER_AGENT = "Mozilla/5.0 (compatible; cpx-vscode/0.1)";

export function activate(context: vscode.ExtensionContext): void {
  sidebar.buildDir = path.join(context.globalStorageUri.fsPath, "build");
  dataDir = path.join(context.globalStorageUri.fsPath, "problems");
  sidebar.katexDir = vscode.Uri.joinPath(context.extensionUri, "node_modules", "katex", "dist");
  context.subscriptions.push(
    vscode.commands.registerCommand("cpx.openProblem", () => openProblem(context)),
    vscode.commands.registerCommand("cpx.runSamples", () => runActive()),
    vscode.commands.registerCommand("cpx.submit", () => submitActive()),
    vscode.commands.registerCommand("cpx.findEditorial", () => findEditorial()),
    vscode.commands.registerCommand("cpx.showStatement", () => vscode.commands.executeCommand("cpx.actions.focus")),
    vscode.window.registerWebviewViewProvider("cpx.actions", sidebar, { webviewOptions: { retainContextWhenHidden: true } }),
    vscode.window.onDidChangeActiveTextEditor(() => sidebar.refresh()),
  );
}

/** Where problem info, tests, and statements are kept (set on activation). */
let dataDir = "";

/** The problem behind a solution file, and where its tests and statement are. */
interface Linked {
  file: string;
  language?: Language;
  stored: { ref: ProblemRef; title: string };
  testsFile: string;
  statementFile: string;
}

function linkedTo(file: string): Linked | undefined {
  for (const meta of [metaFor(dataDir, file), sideMetaFor(file)]) {
    const stored = readStored(meta.problem);
    if (stored) {
      return { file, language: languageOf(path.extname(file)), stored, testsFile: meta.samples, statementFile: meta.statement };
    }
  }
  return undefined;
}

/** The sidebar follows the active editor. Focusing something that is not a
 * linked solution (the sidebar itself, another file) keeps the last problem. */
class Sidebar implements vscode.WebviewViewProvider {
  buildDir = "";
  katexDir?: vscode.Uri;
  private view?: vscode.WebviewView;
  private linked?: Linked;
  private samples: Sample[] = [];
  private results: (CaseResult | undefined)[] = [];
  private compileError?: string;
  private running?: "all" | number;

  resolveWebviewView(view: vscode.WebviewView): void {
    this.view = view;
    view.webview.options = { enableScripts: true, localResourceRoots: this.katexDir ? [this.katexDir] : [] };
    const katexCss = this.katexDir ? view.webview.asWebviewUri(vscode.Uri.joinPath(this.katexDir, "katex.min.css")).toString() : "";
    view.webview.html = sidebarPage(view.webview.cspSource, nonce(), katexCss);
    view.webview.onDidReceiveMessage((m) => void this.onMessage(m));
    this.refresh();
  }

  /** Picks up the active editor's problem, if it has one. */
  refresh(): void {
    const editor = vscode.window.activeTextEditor;
    if (editor && editor.document.uri.scheme === "file") {
      const linked = linkedTo(editor.document.fileName);
      if (linked && linked.file !== this.linked?.file) {
        this.linked = linked;
        this.samples = readSamples(linked.testsFile);
        this.results = [];
        this.compileError = undefined;
      }
    }
    this.post();
  }

  private post(): void {
    if (!this.view) return;
    const l = this.linked;
    const state: ViewState = l
      ? {
          problem: { title: l.stored.title, url: l.stored.ref.url, label: `${platformName(l.stored.ref.platform)} ${l.stored.ref.id}` },
          statementHtml: withExamples(statementView(l.statementFile), this.samples),
          tests: this.samples.map((s, i) => toViewTest(s, this.results[i])),
          compileError: this.compileError,
          running: this.running,
        }
      : { statementHtml: "", tests: [] };
    if (this.compileError) {
      for (const t of state.tests) t.result = { verdict: "CE", timeMs: 0, actual: "", mismatch: -1 };
    }
    void this.view.webview.postMessage({ type: "state", state });
  }

  private async onMessage(m: { type: string; index?: number; tests?: { input: string; expected: string }[] }): Promise<void> {
    switch (m.type) {
      case "ready":
        return this.post();
      case "run":
        return this.run(m.index);
      case "save":
        return this.save(m.tests ?? []);
      case "submit":
        return submitFile(this.linked);
      case "editorial":
        return openEditorialSearch(this.linked);
      case "openProblem":
        await vscode.commands.executeCommand("cpx.openProblem");
        return;
    }
  }

  private save(tests: { input: string; expected: string }[]): void {
    if (!this.linked) return;
    const next: Sample[] = tests.map((t) => ({ Input: t.input, Output: t.expected }));
    // A result stays only while its test is unchanged.
    this.results = next.map((s, i) => {
      const old = this.samples.findIndex((o) => o.Input === s.Input && o.Output === s.Output);
      const r = old === -1 ? undefined : this.results[old];
      return r && { ...r, index: i + 1 };
    });
    this.samples = next;
    fs.writeFileSync(this.linked.testsFile, JSON.stringify(next, null, 2) + "\n");
    this.post();
  }

  /** Runs every test, or only the one at index. */
  async run(index?: number): Promise<void> {
    const l = this.linked;
    if (!l) {
      vscode.window.showErrorMessage("CPX: open a solution file CPX created first.");
      return;
    }
    if (this.running !== undefined) return;
    if (!l.language) {
      vscode.window.showErrorMessage(`CPX: no language runs ${path.extname(l.file)} files. Supported: ${Object.values(SPECS).map((x) => x.ext).filter((e, i, a) => a.indexOf(e) === i).join(", ")}.`);
      return;
    }
    if (this.samples.length === 0) {
      vscode.window.showErrorMessage("CPX: this problem has no tests. Add one in the CPX sidebar.");
      return;
    }
    const doc = vscode.workspace.textDocuments.find((d) => d.fileName === l.file);
    if (doc?.isDirty) await doc.save();

    fs.mkdirSync(this.buildDir, { recursive: true });
    const spec: RunSpec = {
      commands: commandsFor(l.language, settings().get<Record<string, { compile?: string; run?: string }>>("commands")?.[l.language]),
      source: l.file,
      buildDir: this.buildDir,
      timeLimitMs: settings().get<number>("timeLimitMs") ?? 5000,
    };
    this.running = index ?? "all";
    if (index === undefined) this.results = [];
    else this.results[index] = undefined;
    this.compileError = undefined;
    this.post();
    try {
      if (index === undefined) {
        this.results = await runSamples(spec, this.samples);
      } else {
        const [r] = await runSamples(spec, [this.samples[index]]);
        this.results[index] = { ...r, index: index + 1 };
      }
    } catch (e) {
      this.compileError = (e as Error).message.replace(/^Compilation failed:\n?/, "");
    } finally {
      this.running = undefined;
      this.post();
    }
  }
}

/** The saved HTML statement when there is one (newer problems), else the plain text. */
function statementView(textFile: string): string {
  const html = readStatement(htmlStatementPath(textFile));
  return html !== undefined ? statementHtmlWithMath(html) : statementHtml(readStatement(textFile) ?? "");
}

function platformName(p: string): string {
  return p === "codeforces" ? "Codeforces" : p === "cses" ? "CSES" : p === "atcoder" ? "AtCoder" : p;
}

function nonce(): string {
  return Array.from({ length: 32 }, () => Math.floor(Math.random() * 36).toString(36)).join("");
}

const sidebar = new Sidebar();

export function deactivate(): void {}

function settings(): vscode.WorkspaceConfiguration {
  return vscode.workspace.getConfiguration("cpx");
}

function root(): string {
  const first = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  return solutionRoot(settings().get<string>("workspaceFolder"), first);
}

// Uses https.get rather than fetch: Codeforces answers Node's fetch with 403
// whatever the User-Agent, but accepts a plain https request.
export function fetchPage(url: string, redirects = 5): Promise<string> {
  return new Promise((resolve, reject) => {
    const req = https.get(url, { headers: { "User-Agent": USER_AGENT, "Accept-Language": "en" } }, (res) => {
      const status = res.statusCode ?? 0;
      if (status >= 300 && status < 400 && res.headers.location && redirects > 0) {
        res.resume();
        resolve(fetchPage(new URL(res.headers.location, url).toString(), redirects - 1));
        return;
      }
      if (status < 200 || status >= 300) {
        res.resume();
        reject(new Error(`GET ${url} returned ${status}`));
        return;
      }
      res.setEncoding("utf8");
      let body = "";
      res.on("data", (chunk: string) => (body += chunk));
      res.on("end", () => resolve(body));
    });
    req.setTimeout(30_000, () => req.destroy(new Error(`GET ${url} timed out`)));
    req.on("error", reject);
  });
}

function parseFor(ref: ProblemRef, html: string): ParsedProblem {
  switch (ref.platform) {
    case "codeforces":
      return parseCodeforces(html, ref.url);
    case "cses":
      return parseCSES(html, ref.url);
    case "atcoder":
      return parseAtCoder(html, ref.url);
  }
}

async function openProblem(context: vscode.ExtensionContext): Promise<void> {
  const raw = await vscode.window.showInputBox({
    prompt: "Problem URL (Codeforces, CSES, or AtCoder)",
    placeHolder: "https://codeforces.com/problemset/problem/4/A",
  });
  if (!raw) return;

  const ref = parseProblemUrl(raw);
  if (!ref) {
    vscode.window.showErrorMessage("CPX: that URL is not a Codeforces, CSES, or AtCoder problem.");
    return;
  }

  try {
    await vscode.window.withProgress(
      { location: vscode.ProgressLocation.Notification, title: `CPX: loading ${ref.id}…` },
      async () => {
        const parsed = parseFor(ref, await fetchPage(ref.url));
        const language = defaultLanguage();
        const paths = pathsFor(root(), ref, parsed.title, language, dataDir, settings().get<boolean>("subfolderPerPlatform") ?? false);
        scaffold(paths.solution, language, settings().get<Record<string, string>>("templates")?.[language]);
        writeProblemFiles(paths, { ref, title: parsed.title }, parsed.samples, parsed.statement, parsed.statementHtml);

        const doc = await vscode.workspace.openTextDocument(paths.solution);
        await vscode.window.showTextDocument(doc, { preview: false });
        sidebar.refresh();
        void vscode.commands.executeCommand("cpx.actions.focus");
        vscode.window.showInformationMessage(
          `CPX: ${parsed.samples.length} sample(s) saved for ${ref.id}. Write your solution, then run CPX: Run Samples.`,
        );
      },
    );
  } catch (e) {
    vscode.window.showErrorMessage(`CPX: could not load the problem: ${(e as Error).message}`);
  }
}

/** The problem files that sit beside the active solution file. */
function filesFor(solutionFile: string): { stem: string; dir: string; ext: string } {
  const dir = path.dirname(solutionFile);
  const ext = path.extname(solutionFile);
  const stem = path.basename(solutionFile, ext);
  return { stem, dir, ext };
}

function defaultLanguage(): Language {
  const l = settings().get<string>("defaultLanguage") ?? "cpp";
  return l in SPECS ? (l as Language) : "cpp";
}

function languageOf(ext: string): Language | undefined {
  return languageForExt(ext, defaultLanguage());
}

async function activeSolution(): Promise<{ file: string; ext: string; dir: string; stem: string } | undefined> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.uri.scheme !== "file") {
    vscode.window.showErrorMessage("CPX: open a solution file first.");
    return undefined;
  }
  await editor.document.save();
  const file = editor.document.fileName;
  return { file, ...filesFor(file) };
}

async function runActive(): Promise<void> {
  const active = await activeSolution();
  if (!active) return;
  sidebar.refresh();
  await vscode.commands.executeCommand("cpx.actions.focus");
  await sidebar.run();
}

async function submitActive(): Promise<void> {
  const active = await activeSolution();
  if (!active) return;
  const linked = linkedTo(active.file);
  if (!linked) {
    vscode.window.showErrorMessage("CPX: this file is not linked to a problem. Open the problem with CPX: Open Problem first.");
    return;
  }
  await submitFile(linked);
}

async function findEditorial(): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  const linked = editor ? linkedTo(editor.document.fileName) : undefined;
  if (!linked) {
    vscode.window.showErrorMessage("CPX: open a solution file CPX created first.");
    return;
  }
  await openEditorialSearch(linked);
}

/** A web search for the problem's editorial. CPX does not fetch solutions itself. */
async function openEditorialSearch(linked: Linked | undefined): Promise<void> {
  if (!linked) return;
  const { ref, title } = linked.stored;
  const q = `${platformName(ref.platform)} ${ref.id} ${title} editorial`;
  await vscode.env.openExternal(vscode.Uri.parse(`https://www.google.com/search?q=${encodeURIComponent(q)}`));
}

async function submitFile(linked: Linked | undefined): Promise<void> {
  if (!linked) return;
  const doc = vscode.workspace.textDocuments.find((d) => d.fileName === linked.file);
  if (doc?.isDirty) await doc.save();
  await vscode.env.clipboard.writeText(fs.readFileSync(linked.file, "utf8"));
  await vscode.env.openExternal(vscode.Uri.parse(submitUrl(linked.stored.ref)));
  vscode.window.showInformationMessage("CPX: code copied. Paste it on the judge page and submit.");
}

