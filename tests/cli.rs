//! End-to-end tests for the `tyla` binary.
//!
//! These drive the compiled CLI the same way a user would: feeding input on
//! stdin and asserting on stdout. `CARGO_BIN_EXE_tyla` is provided by Cargo.

use std::io::Write;
use std::process::{Command, Stdio};

/// Run the `tyla` binary with `args`, feeding `stdin`, and return its stdout.
fn run(args: &[&str], stdin: &str) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tyla"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn tyla");

    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");

    let output = child.wait_with_output().expect("wait for tyla");
    assert!(output.status.success(), "tyla exited with {}", output.status);
    String::from_utf8(output.stdout).expect("utf8 stdout")
}

#[test]
fn latex_math_to_typst() {
    let out = run(&["-d", "l2t"], r"\frac{1}{2} + \sqrt{x^2+y^2}");
    assert!(out.contains("1/2"), "got: {out}");
    assert!(out.contains("sqrt("), "got: {out}");
}

#[test]
fn typst_math_to_latex() {
    let out = run(&["-d", "t2l"], "$frac(1,2) + sqrt(x^2+y^2)$");
    assert!(out.contains(r"\frac{1}{2}"), "got: {out}");
    assert!(out.contains(r"\sqrt{"), "got: {out}");
}

#[test]
fn detect_reports_latex() {
    let out = run(&["--detect"], r"\alpha + \beta");
    assert_eq!(out.trim(), "latex");
}

#[test]
fn info_subcommand_prints_version() {
    let out = run(&["info"], "");
    assert!(out.contains("tyla"), "got: {out}");
    assert!(out.contains(env!("CARGO_PKG_VERSION")), "got: {out}");
}

// ---- T2L paper-conversion regressions (formerly shell-script fix-ups) ----

#[test]
fn subscript_does_not_swallow_paren_argument() {
    // `X_t(i)` must be `X_t(i)`, not `X_{t(i)}`. Explicit grouping is preserved.
    let out = run(&["-d", "t2l", "--no-preamble"], "$M_t(i)$ and $R_i(B_i)$");
    assert!(out.contains(r"M_t\left(i\right)"), "got: {out}");
    assert!(out.contains(r"R_i\left(B_i\right)"), "got: {out}");
    let grouped = run(&["-d", "t2l", "--no-preamble"], "$M_(t(i))$");
    assert!(grouped.contains(r"M_{t\left(i\right)}"), "got: {grouped}");
}

#[test]
fn apostrophes_are_preserved() {
    let out = run(&["-d", "t2l", "--no-preamble"], "the harvester's rows");
    assert!(out.contains("harvester's"), "got: {out}");
}

#[test]
fn heading_has_no_leading_space() {
    let out = run(&["-d", "t2l", "--no-preamble"], "= Introduction\n\ntext");
    assert!(out.contains(r"\section{Introduction}"), "got: {out}");
    assert!(!out.contains(r"\section{ "), "leading space not stripped: {out}");
}

#[test]
fn labeled_display_math_becomes_equation_env() {
    let out = run(&["-d", "t2l", "--no-preamble"], "$ a = b $ <eq-foo>");
    assert!(out.contains(r"\begin{equation}"), "got: {out}");
    assert!(out.contains(r"\label{eq-foo}"), "got: {out}");
    assert!(out.contains(r"\end{equation}"), "got: {out}");
    // label must be inside the environment
    let begin = out.find(r"\begin{equation}").unwrap();
    let label = out.find(r"\label{eq-foo}").unwrap();
    let end = out.find(r"\end{equation}").unwrap();
    assert!(begin < label && label < end, "label not inside env: {out}");
}

#[test]
fn at_key_splits_into_ref_vs_cite() {
    // `@fig-x` is defined as a label -> \ref; `@smith2020` is not -> \cite.
    let src = "#figure(image(\"x.png\"), caption: [c]) <fig-x>\n\nsee @fig-x and @smith2020.";
    let out = run(&["-d", "t2l", "--no-preamble"], src);
    assert!(out.contains(r"\ref{fig-x}"), "got: {out}");
    assert!(out.contains(r"\cite{smith2020}"), "got: {out}");
}

#[test]
fn adjacent_citations_are_grouped() {
    let out = run(&["-d", "t2l", "--no-preamble"], "text @a, @b, @c here.");
    assert!(out.contains(r"\cite{a,b,c}"), "got: {out}");
}

#[test]
fn elsarticle_template_becomes_frontmatter() {
    let src = "#import \"@preview/elsearticle:3.1.0\": *\n\
        #show: elsearticle.with(\n\
        title: \"My Title\",\n\
        authors: ((name: [Jane Doe], affiliations: (\"a1\",), corresponding: true, email: \"jane@x.org\"),),\n\
        affiliations: (\"a1\": [Some University, Country]),\n\
        abstract: [The abstract text.],\n\
        keywords: (\"alpha\", \"beta\"),\n\
        )\n\n\
        = Introduction\n\nBody text.";
    let out = run(&["-d", "t2l", "-f"], src);
    assert!(out.contains(r"\documentclass[review,12pt]{elsarticle}"), "got: {out}");
    assert!(out.contains(r"\begin{frontmatter}"), "got: {out}");
    assert!(out.contains(r"\title{My Title}"), "got: {out}");
    assert!(out.contains(r"\author[a1]{Jane Doe\corref{cor1}}"), "got: {out}");
    assert!(out.contains(r"\ead{jane@x.org}"), "got: {out}");
    assert!(out.contains("alpha \\sep beta"), "got: {out}");
    assert!(out.contains(r"\section{Introduction}"), "got: {out}");
}

#[test]
fn ieee_template_becomes_authorblocks() {
    let src = "#import \"@preview/charged-ieee:0.1.3\": ieee\n\
        #show: ieee.with(\n\
        title: [A Title],\n\
        authors: ((name: \"Jane Doe\", organization: [Uni], location: [Country], email: \"j@x.org\"),),\n\
        abstract: [Abstract.],\n\
        index-terms: (\"x\", \"y\"),\n\
        )\n\n= Intro\n\nBody.";
    let out = run(&["-d", "t2l", "-f"], src);
    assert!(out.contains(r"\documentclass[conference]{IEEEtran}"), "got: {out}");
    assert!(out.contains(r"\IEEEauthorblockN{Jane Doe}"), "got: {out}");
    assert!(out.contains(r"\begin{IEEEkeywords}"), "got: {out}");
}

#[test]
fn lovelace_pseudocode_becomes_algorithm() {
    let src = "#figure(kind: \"algorithm\", caption: [Demo.], pseudocode-list(booktabs: true)[\n\
        + set $x <- 0$\n\
        + *while* $x < 10$ *do*\n  + increment $x$\n+ *end while*\n\
        ]) <alg-demo>";
    let out = run(&["-d", "t2l", "--no-preamble"], src);
    assert!(out.contains(r"\begin{algorithm}"), "got: {out}");
    assert!(out.contains(r"\begin{algorithmic}[1]"), "got: {out}");
    assert!(out.contains(r"\While{$x < 10$}"), "got: {out}");
    assert!(out.contains(r"\State increment $x$"), "got: {out}");
    assert!(out.contains(r"\EndWhile"), "got: {out}");
    // label is pulled inside the float, after the caption
    let label = out.find(r"\label{alg-demo}").expect("label present");
    let endalg = out.find(r"\end{algorithm}").expect("end present");
    assert!(label < endalg, "label must be inside algorithm: {out}");
}

#[test]
fn wide_table_uses_wrapping_columns() {
    let src = "#table(columns: 2,\n\
        [Short], [A fairly long descriptive sentence that should wrap within its column nicely],\n\
        )";
    let out = run(&["-d", "t2l", "--no-preamble"], src);
    assert!(out.contains(r"\begin{tabular}{|p{"), "expected p{{}} columns, got: {out}");
}

// ---- L2T (LaTeX -> Typst) reverse-direction parity ----

#[test]
fn l2t_elsarticle_becomes_show_rule() {
    let src = "\\documentclass[review,12pt]{elsarticle}\n\\begin{document}\n\\begin{frontmatter}\n\
        \\title{My Title}\n\\author[a1]{Jane Doe\\corref{cor1}}\n\\ead{jane@x.org}\n\
        \\affiliation[a1]{organization={Some University}, country={Country}}\n\
        \\begin{abstract}\nThe abstract.\n\\end{abstract}\n\
        \\begin{keyword}\nalpha \\sep beta\n\\end{keyword}\n\\end{frontmatter}\n\
        \\section{Intro}\nBody.\n\\end{document}";
    let out = run(&["-d", "l2t"], src);
    assert!(out.contains("#import \"@preview/elsearticle"), "got: {out}");
    assert!(out.contains("#show: elsearticle.with("), "got: {out}");
    assert!(out.contains("title: [My Title]"), "got: {out}");
    assert!(out.contains("corresponding: true"), "got: {out}");
    assert!(out.contains("email: \"jane@x.org\""), "got: {out}");
    assert!(out.contains("\"a1\": [Some University, Country]"), "got: {out}");
}

#[test]
fn l2t_algorithm_becomes_pseudocode_list() {
    let src = "\\begin{algorithm}\n\\caption{Demo}\n\\label{alg-x}\n\\begin{algorithmic}[1]\n\
        \\State init\n\\While{$x < 10$}\n\\State step\n\\EndWhile\n\\end{algorithmic}\n\\end{algorithm}";
    let out = run(&["-d", "l2t", "--no-preamble"], src);
    assert!(out.contains("kind: \"algorithm\""), "got: {out}");
    assert!(out.contains("pseudocode-list(booktabs: true)"), "got: {out}");
    assert!(out.contains("*while*") && out.contains("*do*"), "got: {out}");
    assert!(out.contains("*end while*"), "got: {out}");
    assert!(out.contains("<alg-x>"), "got: {out}");
}

#[test]
fn l2t_multi_cite_uses_bare_refs() {
    // `\cite{a,b}` must be `@a @b`, never the invalid `#cite(<a>, <b>)`.
    let out = run(&["-d", "l2t", "--no-preamble"], "text \\cite{a2020,b2021} end");
    assert!(out.contains("@a2020 @b2021"), "got: {out}");
    assert!(!out.contains("#cite(<a2020>, <b2021>)"), "got: {out}");
}

#[test]
fn l2t_bibliography_becomes_typst_call() {
    let src = "\\documentclass{article}\n\\begin{document}\nText.\n\\bibliographystyle{plain}\n\
        \\bibliography{references}\n\\end{document}";
    let out = run(&["-d", "l2t"], src);
    assert!(out.contains("#bibliography(\"references.bib\")"), "got: {out}");
    assert!(!out.contains("bibliographystyle"), "leaked style: {out}");
}
