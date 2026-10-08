// Problem identity and the file names CPX uses. These match the Go core's
// rules, so the terminal app and the extension find the same files.

export type Platform = "codeforces" | "cses" | "atcoder";

export interface ProblemRef {
  platform: Platform;
  id: string;
  url: string;
}

/** Reads a judge URL. Returns undefined for anything CPX does not support. */
export function parseProblemUrl(raw: string): ProblemRef | undefined {
  const s = raw.trim();
  let m = s.match(/codeforces\.com\/(?:problemset\/problem|contest|gym)\/(\d+)\/([A-Za-z]\d?)/);
  if (m) {
    const id = m[1] + m[2].toUpperCase();
    return { platform: "codeforces", id, url: `https://codeforces.com/problemset/problem/${m[1]}/${m[2].toUpperCase()}` };
  }
  m = s.match(/cses\.fi\/problemset\/task\/(\d+)/);
  if (m) {
    return { platform: "cses", id: m[1], url: `https://cses.fi/problemset/task/${m[1]}` };
  }
  m = s.match(/atcoder\.jp\/contests\/([^/]+)\/tasks\/([^/?#]+)/);
  if (m) {
    return { platform: "atcoder", id: m[2], url: `https://atcoder.jp/contests/${m[1]}/tasks/${m[2]}` };
  }
  return undefined;
}

/** Codeforces uses the id (4A.cpp). CSES uses a PascalCase title. AtCoder uses the lowercased id. */
export function fileBaseName(ref: ProblemRef, title: string): string {
  switch (ref.platform) {
    case "codeforces":
      return ref.id;
    case "cses":
      return pascal(title) || ref.id;
    default:
      return ref.id.toLowerCase();
  }
}

/** "Weird Algorithm" becomes "WeirdAlgorithm". Non-alphanumerics split words. */
export function pascal(title: string): string {
  return title
    .split(/[^\p{L}\p{N}]+/u)
    .filter((w) => w.length > 0)
    .map((w) => w[0].toUpperCase() + w.slice(1))
    .join("");
}

/** The judge page where a solution is pasted and submitted by hand. */
export function submitUrl(ref: ProblemRef): string {
  switch (ref.platform) {
    case "codeforces": {
      const m = ref.id.match(/^(\d+)([A-Z]\d?)$/);
      return m ? `https://codeforces.com/problemset/submit/${m[1]}/${m[2]}` : ref.url;
    }
    case "cses":
      return `https://cses.fi/problemset/submit/${ref.id}`;
    case "atcoder": {
      const contest = ref.id.split("_")[0];
      return `https://atcoder.jp/contests/${contest}/submit?taskScreenName=${ref.id}`;
    }
  }
}
