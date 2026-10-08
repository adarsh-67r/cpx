# CPX

Competitive programming in the terminal. Browse problems from Codeforces,
CSES, and AtCoder, get practice picks, read statements, run your solution
against the samples, and submit, without leaving the keyboard.

CPX has two parts:

- **`rs/`**: the terminal app (Rust, ratatui).
- **`extensions/vscode/`**: a standalone VS Code extension. It does not need the terminal app.

## Features

- **Problems**: every Codeforces, CSES, and AtCoder problem, filterable with `/`.
- **Practice**: seven pick modes (auto, weakness, push, refresh, upsolve, explore, plan), each pick with the reason it was chosen.
- **Statements**: problem statements rendered in the terminal, with math kept readable.
- **Run**: compile and run your solution against the samples, with per-sample pass/fail and timing.
- **Submit**: copies your code to the clipboard and opens the judge's submit page.
- **Contests**: upcoming and running Codeforces contests with countdowns.
- **Analytics and Dashboard**: rating history, activity heatmap, topic breakdown, streaks, recent submissions.
- **Config**: handle, language, workspace folder, editor, and theme (Catppuccin), editable in the app.

## Terminal app

Requires [Rust](https://rustup.rs) and a C compiler (SQLite is built from source).

```sh
cd rs
cargo build --release
./target/release/cpx          # first run asks a few setup questions
```

```sh
cpx          # open the app
cpx sync     # fetch problems, contests, and your submissions into the local cache
cpx setup    # run the setup questions again
```

Keys: `j/k` move · `tab` switch tab · `/` filter · `m` practice mode · `o` open in editor ·
`t` run samples · `s` submit · `v` statement · `d/u` scroll · `b` open in browser · `q` quit.

Config and the cache live in your OS config folder (`config.json`, `cache.db`).

## VS Code extension

```sh
cd extensions/vscode
npm install
npm run build
```

Open the folder in VS Code and press F5 to launch it. Commands: **CPX: Open Problem
from URL**, **Run Samples for Active File**, **Copy Code and Open Submit Page**, **Show Statement**.

## Tests

```sh
cd rs && cargo test
cd extensions/vscode && npm test
```

Judge parsers are tested against saved pages in `rs/tests/fixtures/` and
`extensions/vscode/test/fixtures/`.

## License

MIT
