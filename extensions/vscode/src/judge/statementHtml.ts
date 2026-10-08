// The statement as safe HTML: structure, lists, tables, and images survive,
// math becomes $…$ / $$…$$ text, and anything outside a short allowlist of
// tags and attributes is dropped. Links and images must be http(s) (or data:
// images); relative ones are resolved against the page URL.

import * as cheerio from "cheerio";

const KEEP = new Set([
  "p", "div", "span", "br", "hr", "ul", "ol", "li", "b", "strong", "i", "em", "u", "s", "sub", "sup", "small",
  "code", "pre", "blockquote", "center", "table", "thead", "tbody", "tr", "td", "th", "img", "a",
  "h1", "h2", "h3", "h4", "h5", "h6", "section", "dl", "dt", "dd", "figure", "figcaption",
]);
const DROP = "script, style, iframe, object, embed, link, meta, form, input, button, svg, noscript, .MathJax, .MathJax_Preview, mjx-container";
const ATTRS: Record<string, string[]> = {
  img: ["src", "alt", "width", "height"],
  a: ["href"],
  td: ["colspan", "rowspan"],
  th: ["colspan", "rowspan"],
  p: ["data-limits"], // set by cleanStatement itself, styled as the limits line
};

function safeUrl(raw: string | undefined, base: string, allowData: boolean): string | undefined {
  if (!raw) return undefined;
  if (allowData && /^data:image\/(png|jpe?g|gif|webp);/i.test(raw)) return raw;
  try {
    const u = new URL(raw, base);
    return u.protocol === "https:" || u.protocol === "http:" ? u.toString() : undefined;
  } catch {
    return undefined;
  }
}

/** Math in any of the judges' forms becomes plain $…$ / $$…$$ text. */
function mathToText($: cheerio.CheerioAPI, root: cheerio.Cheerio<any>): void {
  root.find("script[type^='math/tex']").each((_, el) => {
    const s = $(el);
    const display = (s.attr("type") ?? "").includes("mode=display");
    s.replaceWith(display ? `$$${s.text()}$$` : `$${s.text()}$`);
  });
  root.find(".math").each((_, el) => {
    const tex = ($(el).find("annotation").first().text() || $(el).text()).trim();
    if (tex) $(el).replaceWith($(el).hasClass("math-display") ? `$$${tex}$$` : `$${tex}$`);
  });
  root.find("var").each((_, el) => {
    $(el).replaceWith(`$${$(el).text().trim()}$`);
  });
}

/** Cleans root in place and returns its inner HTML. pageUrl resolves relative links. */
export function cleanStatement($: cheerio.CheerioAPI, root: cheerio.Cheerio<any>, pageUrl: string): string {
  mathToText($, root);
  root.find(DROP).remove();
  // Codeforces header (title, limits, file names) becomes one line of limits;
  // the title is shown separately.
  root.find(".header").each((_, el) => {
    const limit = (cls: string) => $(el).find(cls).clone().find(".property-title").remove().end().text().trim();
    const parts = [limit(".time-limit"), limit(".memory-limit")].filter(Boolean);
    $(el).replaceWith(parts.length ? `<p data-limits>${parts.join(" · ")}</p>` : "");
  });
  // Section titles become one heading level whatever the page used (CSES uses
  // h1, AtCoder h3, Codeforces a .section-title div).
  root.find("h1, h2, h4, h5, h6").each((_, el) => {
    $(el).replaceWith(`<h3>${$(el).html() ?? ""}</h3>`);
  });
  // Codeforces section titles ("Input", "Output", "Note") become headings.
  root.find(".section-title").each((_, el) => {
    $(el).replaceWith(`<h3>${$(el).html() ?? ""}</h3>`);
  });
  // Deepest first, so unwrapping a parent never skips its children.
  root.find("*").get().reverse().forEach((el: any) => {
    const tag = (el.tagName ?? "").toLowerCase();
    const node = $(el);
    if (!KEEP.has(tag)) {
      node.replaceWith(node.contents());
      return;
    }
    const allowed = ATTRS[tag] ?? [];
    for (const name of Object.keys(el.attribs ?? {})) {
      if (!allowed.includes(name)) node.removeAttr(name);
    }
    if (tag === "img") {
      const src = safeUrl(node.attr("src"), pageUrl, true);
      if (src) node.attr("src", src);
      else node.remove();
    }
    if (tag === "a") {
      const href = safeUrl(node.attr("href"), pageUrl, false);
      if (href) node.attr("href", href);
      else node.removeAttr("href");
    }
  });
  // Codeforces writes math as $$$…$$$ inline and $$$$$$…$$$$$$ display.
  return (root.html() ?? "")
    .replace(/(<hr>\s*){2,}/g, "<hr>") // removed sample sections leave their separators behind
    .replace(/(\s*<hr>)+\s*$/, "")
    .replace(/\${6}([\s\S]+?)\${6}/g, "$$$$$1$$$$").replace(/\${3}([\s\S]+?)\${3}/g, "$$$1$$").trim();
}
