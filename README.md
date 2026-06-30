# tyla

**Bidirectional LaTeX ↔ Typst converter.** Write in one, get the other — and round‑trip back without losing your structure.

`tyla` converts whole documents, not just math fragments: section structure, equations and labels, citations and cross‑references, tables, figures, algorithms, and paper‑template front matter all survive the trip in both directions.

```
   main.typ  ──tyla -d t2l──▶  main.tex   (compile with tectonic/pdflatex)
   main.tex  ──tyla -d l2t──▶  main.typ   (compile with typst)
```

---

## Why

Most converters handle math and give up on the rest, so you end up hand‑patching the output with a wall of `sed`/`perl`. `tyla` aims to make the conversion good enough to compile as‑is:

- **Two directions, real parity.** A `typ → tex → typ` (or `tex → typ → tex`) round‑trip keeps the same structure and renders the same. Headings stay headings, grouped citations stay grouped, equation labels stay attached.
- **Paper templates, not just markup.** A Typst `#show: elsearticle.with(...)` / `ieee.with(...)` becomes a proper `\documentclass{elsarticle}` / `IEEEtran` document with full front matter (authors, affiliations, abstract, keywords, journal) — and back again.
- **Algorithms.** Lovelace `pseudocode-list` ↔ `algorithm` / `algorithmic`, including `\While`/`\For`/`\If`/`\Procedure`, comments, and nesting.
- **Smart references.** A Typst `@key` is split into `\cite{}` vs `\ref{}` automatically by looking at which labels the document actually defines — no flags, no `.bib` parsing required.
- **The small stuff that breaks compiles.** Possessive apostrophes, non‑breaking spaces before references, subscript scoping (`X_t(i)` not `X_{t(i)}`), display‑math labels, and wrapping table columns.

## Install

From source (requires a Rust toolchain):

```sh
cargo install --path .
# or
cargo build --release   # binary at target/release/tyla
```

## Usage

The default action is convert; direction is auto‑detected from the file extension or content, or set explicitly with `-d`.

```sh
# Math snippet, Typst -> LaTeX
printf '$frac(1, 2) + sqrt(x^2)$' | tyla -d t2l
# -> $\frac{1}{2} + \sqrt{x^2}$

# Math snippet, LaTeX -> Typst  (--no-preamble keeps it bare)
printf '\frac{1}{2} + \sqrt{x^2}' | tyla -d l2t --no-preamble
# -> 1/2 + sqrt(x^(2))

# Whole document (front matter, sections, equations, the lot)
tyla -d t2l -f paper.typ -o paper.tex      # Typst -> LaTeX
tyla -d l2t -f paper.tex -o paper.typ      # LaTeX -> Typst
```

Without `-f`, input is treated as a fragment; with `-f` it is converted as a full document. Reading from stdin and writing to stdout (as above) works whenever no files are given.

### Subcommands

| Command | What it does |
|---|---|
| `convert` | Convert a file (the default action) |
| `check` | Report potential LaTeX conversion issues without converting |
| `tikz` | Convert TikZ ↔ CeTZ graphics |
| `batch` | Convert many files (glob/directory in, directory out) |
| `info` | Show version and supported features |

```sh
tyla batch "src/*.tex" -o out/ -d l2t --recursive
tyla --detect paper.typ        # prints "typst" or "latex"
tyla check paper.tex           # lints for conversion gotchas
```

Run `tyla --help` for the full option list (`--pretty`, `--strict`, `--no-preamble`, `--preamble`, `--wrapper`, …).

## As a library

```toml
[dependencies]
tyla = { git = "https://github.com/bresilla/tyla" }
```

```rust
use tyla::{latex_to_typst, typst_to_latex, latex_document_to_typst};

let typst = latex_to_typst(r"\frac{1}{2}");        // math
let latex = typst_to_latex("$frac(1, 2)$");        // math, other way
let doc   = latex_document_to_typst(r"\section{Intro} Hello"); // full document
```

The crate exposes the document/diagnostic APIs too (`typst_to_latex_with_diagnostics`, `latex_to_typst_with_diagnostics`, …) if you want structured warnings instead of stderr output.

## How it works

The two directions use different parsers, which is why each can be tuned independently:

- **Typst → LaTeX** parses with Typst's own `typst-syntax`, evaluates a small subset of Typst scripting, then emits LaTeX.
- **LaTeX → Typst** parses with `mitex`, walks the syntax tree, and emits Typst.

Template, algorithm, and front‑matter handling sits on top of those parsers, so adding a new template or environment rarely means touching the parser itself.

## Building & development

A `Makefile` wraps the common tasks:

```sh
make build        # debug build
make run          # convert the bundled example (examples/sample.tex)
make demo         # round-trip both bundled examples
make test         # run the suite
make harden       # fmt --check + clippy -D warnings + full tests
make help         # list everything
```

There is also a `flake.nix` providing a dev shell with the Rust toolchain; `direnv allow` drops you into it.

## Acknowledgments

`tyla` is a hard fork of [**tylax**](https://github.com/scipenai/tylax) by SciPenAI. The original project built the core conversion engine this fork stands on. See [ACKNOWLEDGMENTS.md](./ACKNOWLEDGMENTS.md) for details.

## License

Apache‑2.0. See [LICENSE](./LICENSE).
