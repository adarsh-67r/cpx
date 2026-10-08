// The results panel. Kept free of VS Code imports so it can be tested.

import { CaseResult } from "./runner";

export function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

export function renderResults(title: string, results: CaseResult[]): string {
  const passed = results.filter((r) => r.passed).length;
  const rows = results
    .map((r) => {
      if (r.passed) {
        return `<div class="ok">&#10003; sample ${r.index} <span class="muted">${r.durationMs} ms</span></div>`;
      }
      const reason = r.error ? escapeHtml(r.error) : "wrong answer";
      const detail = r.error
        ? ""
        : `<div class="diff"><div>input<pre>${escapeHtml(r.input)}</pre></div>` +
          `<div>expected<pre>${escapeHtml(r.expected)}</pre></div>` +
          `<div>got<pre>${escapeHtml(r.actual)}</pre></div></div>`;
      return `<div class="bad">&#10007; sample ${r.index} <span>${reason}</span></div>${detail}`;
    })
    .join("\n");
  return `<!doctype html><html><head><meta charset="utf-8"><style>
body { font-family: var(--vscode-font-family); color: var(--vscode-foreground); padding: 12px; }
.ok { color: #a6e3a1; margin: 4px 0; }
.bad { color: #f38ba8; margin: 8px 0 4px; font-weight: bold; }
.muted { color: var(--vscode-descriptionForeground); font-weight: normal; }
.diff { display: flex; gap: 12px; flex-wrap: wrap; }
.diff > div { flex: 1; min-width: 180px; }
pre { background: var(--vscode-textCodeBlock-background); padding: 6px; border-radius: 4px; white-space: pre-wrap; }
</style></head><body>
<h2>${escapeHtml(title)}</h2>
<p><b>${passed}/${results.length} passed</b></p>
${rows}
</body></html>`;
}
