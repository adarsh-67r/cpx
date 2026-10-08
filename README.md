<div align="center">

# ⚡ CPX

**Competitive programming without leaving your terminal.**

Find a problem, read it, solve it, test it, submit it: one keyboard, zero browser tabs.

[![CI](https://github.com/adarsh-67r/cpx/actions/workflows/ci.yml/badge.svg)](https://github.com/adarsh-67r/cpx/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-cba6f7.svg)](LICENSE)
![Rust](https://img.shields.io/badge/built%20with-Rust-fab387.svg)
![Judges](https://img.shields.io/badge/judges-Codeforces%20·%20CSES%20·%20AtCoder-89b4fa.svg)

</div>

```
CPX  [Problems]  Practice   Contests   Analytics   Dashboard   Config    21498/21498
╭───────────────────────────────────────────────────╮╭───────────────────────────────────────────────────╮
│ ▸ 1000A    1200   Codehorses T-shirts             ││ Codehorses T-shirts                               │
│   1000B    1500   Light It Up                     ││ codeforces · 1000A · rating 1200                  │
│   1000C    1700   Covered Points Count            ││ https://codeforces.com/problemset/problem/1000/A  │
│   1000D    1900   Yet Another Problem On a Subse… ││ greedy, implementation                            │
│   1000E    2100   We Need More Bosses             ││                                                   │
│   1000F    2400   One Occurrence                  ││ Last run                                          │
│   1000G    2700   Two-Paths                       ││ ✓ sample 1  4ms                                   │
│   1001A    1100   Generate plus state or minus s… ││ ✓ sample 2  3ms                                   │
╰───────────────────────────────────────────────────╯╰───────────────────────────────────────────────────╯
j/k move · tab switch · / filter · m mode · o open · t test · s submit · v statement · q quit
```

## Why CPX?

The usual loop is: browse the problemset in a browser, copy samples by hand, alt-tab to the editor, paste
input into a terminal, squint at the output, copy the code, find the submit page. CPX turns that into
four keys:

| Key | What happens |
|:---:|---|
| `v` | Read the statement right in the terminal, math included |
| `o` | Create the solution file from your template, with the samples saved next to it, and open your editor |
| `t` | Compile and run every sample, with ✓/✗, timing, and expected vs. got |
| `s` | Copy your code and open the judge's submit page |

Everything is cached locally in SQLite, so browsing 20,000+ problems is instant and works offline.

## Features

- 🗂️ **Every problem, one list**: Codeforces, CSES, and AtCoder, filterable with `/`.
- 🎯 **Practice that explains itself**: seven pick modes, and every pick says *why* it was chosen.
- 📖 **Statements in the terminal**: headings, lists, sample boxes, and readable math.
- 🧪 **Local judge**: per-sample verdicts, timing, and time limits that really stop runaway programs.
- 🏆 **Contests**: upcoming and running Codeforces rounds with live countdowns.
- 📈 **Analytics**: rating history, a 12-week activity heatmap, and your strongest and weakest topics.
- 🏠 **Dashboard**: current rating, distance to the next rank, streak, recent verdicts.
- 🎨 **Catppuccin**: Mocha by default, Latte for light terminals.
- 🧩 **VS Code extension**: the same run-and-submit loop inside your editor, no terminal app needed.

### Practice modes

Press `m` in the Practice tab to cycle through the modes. Your skill per topic is the 75th percentile rating of the problems you've solved with that tag.

| Mode | Picks |
|---|---|
| **auto** | Problems near your target rating, with a bonus for weak or untouched topics |
| **weakness** | Problems up to 500 above your skill in one of their topics, biggest stretch first |
| **push** | Up to 300 above your rating, in topics you're already strong in |
| **refresh** | Topics you haven't touched in 60+ days, at a level you've handled before |
| **upsolve** | Problems you tried and never got accepted, newest first |
| **explore** | Topics you've never solved, at or near your level |
| **plan** | The rung just below your next Codeforces rank (1200, 1400, 1600, 1900…) |

## Install

From source (needs [Rust](https://rustup.rs) and a C compiler; SQLite is built in):

```sh
cargo install --git https://github.com/adarsh-67r/cpx
```

Prebuilt binaries for Windows, Linux, and macOS (Intel and Apple Silicon) are attached to each
[release](https://github.com/adarsh-67r/cpx/releases).

## Quick start

```sh
cpx            # first run asks for your Codeforces handle, language, workspace, editor
cpx sync       # pull problems, contests, your submissions and rating into the cache
cpx            # go
```

| Command | |
|---|---|
| `cpx` | Open the app |
| `cpx sync` | Refresh problems, contests, submissions, and ratings |
| `cpx setup` | Run the setup questions again |

**Keys:** `j/k` move · `tab` switch tab · `/` filter · `m` practice mode · `o` open · `t` test ·
`s` submit · `v` statement · `d/u` scroll · `b` browser · `q` quit

Settings live in `config.json` in your OS config folder (`%APPDATA%\cpx` on Windows,
`~/.config/cpx` on Linux, `~/Library/Application Support/cpx` on macOS) and can be edited
from the Config tab. Compile and run commands are templates, so any language works:

```json
"cpp": { "compile": "g++ -std=c++17 -O2 -o {output} {source}", "run": "{dir}/{output}", "extension": ".cpp" }
```

> **CSES progress:** paste your CSES session cookie in the Config tab and `cpx sync` will mark your solved tasks.

## VS Code extension

A standalone extension in [`extensions/vscode`](extensions/vscode). It doesn't need the terminal app.

- **CPX: Open Problem from URL**: paste a Codeforces, CSES, or AtCoder link to get a solution file with its samples
- **CPX: Run Samples for Active File**: a results panel with per-sample verdicts
- **CPX: Show Statement**: the problem beside your code
- **CPX: Copy Code and Open Submit Page**

```sh
cd extensions/vscode && npm install && npm run build   # then press F5 in VS Code
```

## Fair play

CPX never logs in or submits on your behalf. Submitting is always you, in your browser. It reads public
problem pages and the official Codeforces API, and caches what it fetches to keep requests to a minimum.

## Contributing

```sh
cargo test                              # terminal app
cd extensions/vscode && npm test        # extension
```

Judge parsers are tested against saved pages in [`tests/fixtures`](tests/fixtures). When a site changes
its layout, save the new page there, fix the parser, and the test proves it. Issues and PRs welcome.

## License

[MIT](LICENSE)
