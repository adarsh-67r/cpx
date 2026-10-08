// Parsers for judge problem pages. Each returns the title, the sample tests,
// and the statement as plain text and as cleaned HTML.

import * as cheerio from "cheerio";
import { cleanStatement } from "./statementHtml";

export interface Sample {
  Input: string;
  Output: string;
}

export interface ParsedProblem {
  title: string;
  samples: Sample[];
  statement: string;
  /** Safe HTML with $…$ math, or "" when the page had no statement. */
  statementHtml: string;
}

/** Turns a <pre> into text: <br> becomes a newline, and surrounding blank lines go. */
function preText($: cheerio.CheerioAPI, el: cheerio.Cheerio<any>): string {
  const clone = el.clone();
  clone.find("br").replaceWith("\n");
  return clone.text().replace(/\r\n/g, "\n").replace(/^\n+|\n+$/g, "");
}

/** Statement text: block elements end lines, and TeX math is kept between $ signs. */
function statementText($: cheerio.CheerioAPI, root: cheerio.Cheerio<any>): string {
  const clone = root.clone();
  clone.find("script[type^='math/tex']").each((_, el) => {
    const s = $(el);
    const display = (s.attr("type") ?? "").includes("mode=display");
    const tex = s.text();
    s.replaceWith(display ? `\n$$${tex}$$\n` : `$${tex}$`);
  });
  // CSES: .math spans hold raw TeX (or a rendered tree with a TeX annotation).
  clone.find(".math").each((_, el) => {
    const tex = ($(el).find("annotation").first().text() || $(el).text()).trim();
    if (tex) {
      const display = $(el).hasClass("math-display");
      $(el).replaceWith(display ? `\n$$${tex}$$\n` : `$${tex}$`);
    }
  });
  // AtCoder: variables and formulas are <var> elements.
  clone.find("var").each((_, el) => {
    $(el).replaceWith(`$${$(el).text().trim()}$`);
  });
  clone.find("style, script, .MathJax, .MathJax_Preview, mjx-container").remove();
  clone.find("br").replaceWith("\n");
  clone.find("p, div, li, h1, h2, h3, h4, ul, ol, pre, tr, section").each((_, el) => {
    $(el).prepend("\n").append("\n");
  });
  // Codeforces writes math as $$$…$$$ inline and $$$$$$…$$$$$$ display.
  const text = clone
    .text()
    .replace(/\${6}([\s\S]+?)\${6}/g, "\n$$$$$1$$$$\n")
    .replace(/\${3}([\s\S]+?)\${3}/g, "$$$1$$");
  const lines = text.replace(/\r/g, "").split("\n").map((l) => l.replace(/[ \t]+/g, " ").trim());
  return lines.join("\n").replace(/\n{3,}/g, "\n\n").trim();
}

export function parseCodeforces(html: string, url = "https://codeforces.com/"): ParsedProblem {
  const $ = cheerio.load(html);
  const title = $(".title").first().text().replace(/^[A-Z]\d?\.\s*/, "").trim();

  const samples: Sample[] = [];
  $(".sample-test").each((_, st) => {
    const ins = $(st).find(".input pre");
    const outs = $(st).find(".output pre");
    for (let i = 0; i < Math.min(ins.length, outs.length); i++) {
      samples.push({ Input: preText($, ins.eq(i)), Output: preText($, outs.eq(i)) });
    }
  });

  const root = $(".problem-statement").first();
  root.find(".sample-tests").remove();
  const statement = root.length ? statementText($, root) : "";
  return { title, samples, statement, statementHtml: root.length ? cleanStatement($, root, url) : "" };
}

export function parseCSES(html: string, url = "https://cses.fi/"): ParsedProblem {
  const $ = cheerio.load(html);
  const title = $("h1").first().text().trim();

  const samples: Sample[] = [];
  const pres = $("#example").nextAll("pre");
  for (let i = 0; i + 1 < pres.length; i += 2) {
    samples.push({ Input: preText($, pres.eq(i)), Output: preText($, pres.eq(i + 1)) });
  }

  const content = $(".content").first();
  let statement = "";
  let statementHtml = "";
  if (content.length) {
    const ex = content.find("#example");
    ex.nextAll().remove();
    ex.remove();
    statement = statementText($, content);
    statementHtml = cleanStatement($, content, url);
  }
  return { title, samples, statement, statementHtml };
}

export function parseAtCoder(html: string, url = "https://atcoder.jp/"): ParsedProblem {
  const $ = cheerio.load(html);
  const title = $("title").first().text().split(" - ").slice(1).join(" - ").trim() || $("title").text().trim();

  const inputs = new Map<number, string>();
  const outputs = new Map<number, string>();
  $("h3").each((_, h) => {
    const label = $(h).text().trim();
    const parts = label.split(/\s+/);
    const n = Number(parts[parts.length - 1]);
    if (!Number.isInteger(n)) return;
    const pre = $(h).parent().find("pre").first();
    if (!pre.length) return;
    const text = preText($, pre);
    if (label.startsWith("Sample Input") || label.startsWith("入力例")) inputs.set(n, text);
    else if (label.startsWith("Sample Output") || label.startsWith("出力例")) outputs.set(n, text);
  });

  const samples: Sample[] = [...inputs.keys()]
    .filter((n) => outputs.has(n))
    .sort((a, b) => a - b)
    .map((n) => ({ Input: inputs.get(n)!, Output: outputs.get(n)! }));

  const root = $("#task-statement").first();
  const en = root.find(".lang-en");
  const part = en.length ? en.first() : root;
  const statement = root.length ? statementText($, part) : "";
  // Samples are shown in the Tests tab, so their sections leave the statement.
  part.find("h3").each((_, h) => {
    if (!/^(Sample|入力例|出力例)/.test($(h).text().trim())) return;
    const part = $(h).closest(".part");
    // Older pages have no .part wrapper: the heading's parent is the section box.
    (part.length ? part : $(h).parent()).not(root).remove();
  });
  return { title, samples, statement, statementHtml: root.length ? cleanStatement($, part, url) : "" };
}
