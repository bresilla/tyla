#!/usr/bin/env bash
#
# Fetch a corpus of public elsarticle / IEEE template documents for testing the
# converter against real templates. These are upstream template/example files,
# not vendored into the repo (so their licenses stay with their projects); run
# this script to populate ./typst and ./latex.
set -e
cd "$(dirname "$0")"
mkdir -p typst latex

PKG=https://raw.githubusercontent.com/typst/packages/main/packages/preview
CTAN=https://mirrors.ctan.org/macros/latex/contrib

echo "Typst templates (for Typst -> LaTeX):"
# name=version:bibfile
for spec in \
  "elsearticle/3.1.0/refs.bib" "charged-ieee/0.1.3/refs.bib" \
  "arkheion/0.1.2/bibliography.bib" "ilm/2.1.1/refs.bib" \
  "clean-math-paper/0.2.7/bibliography.bib" "springer-spaniel/0.1.0/sample.bib"; do
  name=${spec%%/*}; rest=${spec#*/}; ver=${rest%%/*}; bib=${rest#*/}
  curl -fsSL "$PKG/$name/$ver/template/main.typ" -o "typst/$name.typ"      && echo "  $name.typ"
  curl -fsSL "$PKG/$name/$ver/template/$bib"      -o "typst/${name}_refs.bib" 2>/dev/null || true
done

echo "LaTeX templates (for LaTeX -> Typst):"
for f in elsarticle-template-num elsarticle-template-harv; do
  curl -fsSL "$CTAN/elsarticle/$f.tex" -o "latex/$f.tex" && echo "  $f.tex"
done
for f in bare_conf bare_jrnl bare_adv; do
  curl -fsSL "$CTAN/IEEEtran/$f.tex" -o "latex/$f.tex" && echo "  $f.tex"
done
curl -fsSL "https://raw.githubusercontent.com/borisveytsman/acmart/master/samples/sample-sigconf.tex" \
  -o latex/acmart-sigconf.tex && echo "  acmart-sigconf.tex"
curl -fsSL "$CTAN/revtex/sample/aps/apssamp.tex" -o latex/revtex-apssamp.tex 2>/dev/null || true
# Robotics/AI venues (mostly LaTeX): NeurIPS-style and the common arXiv style.
curl -fsSL "https://raw.githubusercontent.com/ArmageddonKnight/NeurIPS/main/main.tex" \
  -o latex/neurips.tex 2>/dev/null && echo "  neurips.tex"
curl -fsSL "https://raw.githubusercontent.com/kourgeorge/arxiv-style/master/template.tex" \
  -o latex/arxiv-style.tex 2>/dev/null && echo "  arxiv-style.tex"

echo "done. Run ./run.sh to convert + compile each."
