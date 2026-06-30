#!/usr/bin/env bash
#
# Convert every corpus document with tyla and try to compile the result:
#   typst/*.typ  --(tyla -d t2l)-->  .tex  --(tectonic)-->  pdf
#   latex/*.tex  --(tyla -d l2t)-->  .typ  --(typst)----->  pdf
#
# Requires: a built `tyla` (target/{debug,release}), plus `typst` and `tectonic`
# on PATH. Missing figures are stubbed so only conversion/structure is tested.
set -u
cd "$(dirname "$0")"
ROOT=$(cd ../.. && pwd)
TYLA="${TYLA:-$(ls "$ROOT"/target/release/tyla "$ROOT"/target/debug/tyla 2>/dev/null | head -1)}"
OUT=$(mktemp -d)
PASS=0; FAIL=0

stub_images() { # $1 = dir, $2 = source file listing image()/includegraphics
  local dir="$1"
  grep -ohE 'image\("[^"]*"|includegraphics(\[[^]]*\])?\{[^}]*\}' "$dir"/* 2>/dev/null \
    | sed -E 's/.*[("{]([^"})]*)[")}].*/\1/' | sort -u | while read -r img; do
        [ -z "$img" ] && continue
        mkdir -p "$dir/$(dirname "$img")" 2>/dev/null
        printf '\x89PNG\r\n\x1a\n' > "$dir/$img" 2>/dev/null   # tiny stub
      done
}

echo "tyla: $TYLA"
echo
echo "== Typst -> LaTeX (compile with tectonic) =="
for f in typst/*.typ; do
  [ -e "$f" ] || continue
  name=$(basename "$f" .typ); d="$OUT/t2l_$name"; mkdir -p "$d"
  "$TYLA" -d t2l -f "$f" -o "$d/main.tex" 2>/dev/null
  cp typst/"${name}"_refs.bib "$d/refs.bib" 2>/dev/null
  cp typst/*_refs.bib "$d/" 2>/dev/null
  if ( cd "$d" && tectonic main.tex >/dev/null 2>&1 ); then
    echo "  PASS  $name"; PASS=$((PASS+1))
  else
    echo "  FAIL  $name"; FAIL=$((FAIL+1))
  fi
done

echo
echo "== LaTeX -> Typst (compile with typst) =="
for f in latex/*.tex samples/*.tex; do
  [ -e "$f" ] || continue
  name=$(basename "$f" .tex); d="$OUT/l2t_$name"; mkdir -p "$d"
  "$TYLA" -d l2t "$f" 2>/dev/null > "$d/main.typ"
  stub_images "$d"
  if ( cd "$d" && typst compile main.typ main.pdf >/dev/null 2>&1 ); then
    echo "  PASS  $name"; PASS=$((PASS+1))
  else
    echo "  FAIL  $name  (see: cd $d && typst compile main.typ x.pdf)"; FAIL=$((FAIL+1))
  fi
done

echo
echo "summary: $PASS passed, $FAIL failed   (artifacts in $OUT)"
