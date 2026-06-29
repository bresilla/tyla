# Acknowledgments

`tyla` is a **hard fork** of [**tylax**](https://github.com/scipenai/tylax) by
**SciPenAI** and its contributors.

The original project built the foundation this fork stands on: the bidirectional
LaTeX ↔ Typst conversion engine, the AST-based parsers for both directions, the
extensive symbol/command mapping tables, the TikZ ↔ CeTZ support, and the
diagnostics and batch-conversion machinery. None of that work is ours, and we are
grateful for it.

## Why this is a fork

We forked rather than contributed upstream because we intend to take the project
in a direction that goes beyond the original scope — primarily:

- adding new conversion features, and
- parsing **non-standard LaTeX and Typst packages/plugins** (constructs that are
  not part of the standard set the upstream project targets).

A hard fork gives us room to evolve the architecture and the public API freely
for those goals. This is not a statement about the quality of the original
project — it remains an excellent piece of work, and anyone looking for the
canonical, standards-focused converter should use
[the upstream project](https://github.com/scipenai/tylax).

## Licensing

The original `tylax` is distributed under the **Apache License 2.0**, and this
fork continues under the same license. All original copyright notices are
retained. See [`LICENSE`](./LICENSE) for the full text.

If you use `tyla`, please consider also acknowledging the upstream `tylax`
project, since the bulk of the conversion engine originates there.

---

*Thank you to SciPenAI and every contributor to `tylax`.*
