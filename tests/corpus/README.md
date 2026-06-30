# Conversion corpus

Real-world template documents for exercising the converter end to end, in both
directions, against the actual `elsarticle` / IEEE templates.

The files are **not vendored** — they belong to their upstream projects. Fetch
them on demand:

```sh
./fetch.sh     # download the templates into ./typst and ./latex
./run.sh       # convert each with tyla, then compile the result
```

`run.sh` needs a built `tyla` plus `typst` and `tectonic` on `PATH`. It stubs
missing figures, so a FAIL means a conversion/structure problem, not a missing
asset.

## What's in it

| File | Direction | Source |
|---|---|---|
| `typst/elsearticle.typ` | Typst → LaTeX | `@preview/elsearticle` |
| `typst/charged-ieee.typ` | Typst → LaTeX | `@preview/charged-ieee` |
| `typst/arkheion.typ` | Typst → LaTeX | `@preview/arkheion` (arXiv-style) |
| `typst/ilm.typ` | Typst → LaTeX | `@preview/ilm` |
| `typst/clean-math-paper.typ` | Typst → LaTeX | `@preview/clean-math-paper` |
| `typst/springer-spaniel.typ` | Typst → LaTeX | `@preview/springer-spaniel` |
| `latex/elsarticle-template-{num,harv}.tex` | LaTeX → Typst | CTAN `elsarticle` |
| `latex/bare_{conf,jrnl,adv}.tex` | LaTeX → Typst | CTAN `IEEEtran` |
| `latex/acmart-sigconf.tex` | LaTeX → Typst | ACM `acmart` sample |
| `latex/revtex-apssamp.tex` | LaTeX → Typst | APS `revtex` sample (tutorial) |

## Notes / known asset limitations

These are about file assets, not the conversion:

- The CTAN `elsarticle` templates `\includegraphics{example-image-a}` **without a
  file extension**; Typst's `image()` requires one, so supply
  `example-image-a.png` (or edit the path) before compiling.
- The `elsearticle` Typst template ships **SVG** figures; `pdflatex`/`tectonic`
  cannot embed SVG directly — provide a PDF/PNG version when compiling the LaTeX.

## Bugs this corpus has already caught

Regression tests for each live in `tests/cli.rs`:

- bibliography file taken from a `bibliography("…")` **template argument** (not
  just a body call),
- `\section*{…}` starred (unnumbered) sections,
- `\nonumber` / `\notag` dropped instead of leaking as undefined variables,
- commented-out (`%% \author …`) front matter ignored, empty authors skipped,
- `#import` / `#show` / `#set` and `#x.insert(…)` statements dropped instead of
  leaking as text for non-template Typst documents,
- generic preamble carries theorem/algorithm environments and `natbib`,
- `description` lists: `\item[Term] body` → `/ Term: body`,
- `\pageref{x}` → a valid Typst page-counter query (not invalid `@x` in code).

### Known sample-file caveats (not converter bugs)

- `revtex-apssamp.tex` is a **tutorial** that prints literal LaTeX (`\verb`,
  `\texttt{\cite{#1}}`); no converter should "convert" those examples.
- The `elsearticle` Typst template uses **SVG** figures; the CTAN `elsarticle`
  LaTeX templates `\includegraphics{example-image-a}` **without an extension**.
  Both are figure-asset mismatches, supplied at compile time.
