// CPX for VS Code: open a problem from its URL, run the samples on the
// solution, copy the code to submit it by hand, and read the statement.

import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { parseAtCoder, parseCodeforces, parseCSES, ParsedProblem } from "./judge/parse";
import { ProblemRef, parseProblemUrl, submitUrl } from "./problemRef";
import { renderResults } from "./results";
import { CaseResult, Language, RunSpec, runSamples } from "./runner";
import {
  pathsFor,
  readSamples,
  readStatement,
  readStored,
  scaffold,
  solutionRoot,
  writeProblemFiles,
} from "./workspace";

const USER_AGENT = "Mozilla/5.0 (compatible; cpx-vscode/0.1)";

export function activate(context: vscode.ExtensionContext): void {
  context.subscriptions.push(
    vscode.commands.registerCommand("cpx.openProblem", () => openProblem(context)),
    vscode.commands.registerCommand("cpx.runSamples", () => runActive(context)),
    vscode.commands.registerCommand("cpx.submit", () => submitActive()),
    vscode.commands.registerCommand("cpx.showStatement", () => showStatement()),
  );
}

export function deactivate(): void {}

function settings(): vscode.WorkspaceConfiguration {
  return vscode.workspace.getConfiguration("cpx");
}

function root(): string {
  const first = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  return solutionRoot(settings().get<string>("workspaceFolder"), first);
}

async function fetchPage(url: string): Promise<string> {
  const res = await fetch(url, { headers: { "User-Agent": USER_AGENT, "Accept-Language": "en" } });
  if (!res.ok) {
    throw new Error(`GET ${url} returned ${res.status}`);
  }
  return res.text();
}

function parseFor(ref: ProblemRef, html: string): ParsedProblem {
  switch (ref.platform) {
    case "codeforces":
      return parseCodeforces(html);
    case "cses":
      return parseCSES(html);
    case "atcoder":
      return parseAtCoder(html);
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
        const language = (settings().get<string>("defaultLanguage") ?? "cpp") as Language;
        const paths = pathsFor(root(), ref, parsed.title, language);
        scaffold(paths.solution, language);
        writeProblemFiles(paths, { ref, title: parsed.title }, parsed.samples, parsed.statement);

        const doc = await vscode.workspace.openTextDocument(paths.solution);
        await vscode.window.showTextDocument(doc, { preview: false });
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

function languageOf(ext: string): Language | undefined {
  if (ext === ".cpp") return "cpp";
  if (ext === ".py") return "python";
  return undefined;
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

async function runActive(context: vscode.ExtensionContext): Promise<void> {
  const active = await activeSolution();
  if (!active) return;
  const language = languageOf(active.ext);
  if (!language) {
    vscode.window.showErrorMessage("CPX: only .cpp and .py solutions can be run.");
    return;
  }
  const samples = readSamples(path.join(active.dir, active.stem + ".tests.json"));
  if (samples.length === 0) {
    vscode.window.showErrorMessage("CPX: no samples for this file. Open the problem with CPX: Open Problem first.");
    return;
  }

  const buildDir = path.join(context.globalStorageUri.fsPath, "build");
  fs.mkdirSync(buildDir, { recursive: true });
  const spec: RunSpec = {
    language,
    source: active.file,
    buildDir,
    cppCompiler: settings().get<string>("cppCompiler") ?? "g++",
    pythonPath: settings().get<string>("pythonPath") ?? "python",
    timeLimitMs: settings().get<number>("timeLimitMs") ?? 5000,
  };

  const title = active.stem;
  try {
    const results: CaseResult[] = await vscode.window.withProgress(
      { location: vscode.ProgressLocation.Notification, title: `CPX: running ${title}…` },
      () => runSamples(spec, samples),
    );
    const passed = results.filter((r) => r.passed).length;
    showResults(title, renderResults(title, results));
    vscode.window.showInformationMessage(`CPX: ${passed}/${results.length} sample(s) passed.`);
  } catch (e) {
    const message = (e as Error).message;
    showResults(title, `<pre>${message.replace(/&/g, "&amp;").replace(/</g, "&lt;")}</pre>`);
    vscode.window.showErrorMessage("CPX: the solution did not compile or run.");
  }
}

function showResults(title: string, html: string): void {
  const panel = vscode.window.createWebviewPanel("cpxResults", `CPX: ${title}`, vscode.ViewColumn.Beside, {});
  panel.webview.html = html;
}

async function submitActive(): Promise<void> {
  const active = await activeSolution();
  if (!active) return;
  const stored = readStored(path.join(active.dir, active.stem + ".problem.json"));
  if (!stored) {
    vscode.window.showErrorMessage("CPX: this file is not linked to a problem. Open the problem with CPX: Open Problem first.");
    return;
  }
  const code = fs.readFileSync(active.file, "utf8");
  await vscode.env.clipboard.writeText(code);
  await vscode.env.openExternal(vscode.Uri.parse(submitUrl(stored.ref)));
  vscode.window.showInformationMessage("CPX: code copied. Paste it on the judge page and submit.");
}

async function showStatement(): Promise<void> {
  const active = await activeSolution();
  if (!active) return;
  const text = readStatement(path.join(active.dir, active.stem + ".statement.txt"));
  if (text === undefined) {
    vscode.window.showErrorMessage("CPX: no statement saved for this file. Open the problem with CPX: Open Problem first.");
    return;
  }
  const doc = await vscode.workspace.openTextDocument({ content: text, language: "plaintext" });
  await vscode.window.showTextDocument(doc, { preview: true });
}
