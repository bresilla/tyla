//! Detection and LaTeX rendering of Typst paper-template plugins.
//!
//! Papers written for `@preview/elsearticle` or `@preview/charged-ieee` declare
//! all of their front matter through a single show rule, e.g.
//!
//! ```typst
//! #show: elsearticle.with(title: ..., authors: (...), abstract: ..., ...)
//! ```
//!
//! The generic syntax converter cannot map that to a LaTeX document class and
//! front matter, so this module detects the template call, evaluates its
//! arguments with the MiniEval engine, and assembles a complete, compilable
//! `.tex` document around the converted body — no flags, no hand-written
//! preamble.

use super::context::ConvertContext;
use super::engine::{ContentNode, MiniEval, Value};
use super::markup::convert_content_nodes_to_latex;
use typst_syntax::ast::{self};
use typst_syntax::parse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateKind {
    Elsarticle,
    Ieee,
}

#[derive(Debug, Clone, Default)]
pub struct Author {
    pub name: String,
    pub email: Option<String>,
    pub corresponding: bool,
    /// elsarticle: affiliation ids the author belongs to.
    pub affiliations: Vec<String>,
    /// IEEE: organization / location rendered inline per author.
    pub organization: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Frontmatter {
    pub kind: TemplateKind,
    pub title: String,
    pub authors: Vec<Author>,
    /// elsarticle: id -> affiliation text.
    pub affiliations: Vec<(String, String)>,
    pub abstract_text: String,
    pub keywords: Vec<String>,
    /// elsarticle: the target journal (drives the "submitted to …" footer).
    pub journal: Option<String>,
}

/// Detect a known paper-template show rule and extract its front matter.
/// Returns `None` for documents that do not use a recognised template.
pub fn detect_frontmatter(source: &str) -> Option<Frontmatter> {
    let root = parse(source);
    let markup = root.cast::<ast::Markup>()?;

    // Locate `#show: <template>.with(...)`.
    let mut found: Option<(TemplateKind, ast::Args)> = None;
    for expr in markup.exprs() {
        let ast::Expr::ShowRule(show) = expr else {
            continue;
        };
        let ast::Expr::FuncCall(call) = show.transform() else {
            continue;
        };
        let ast::Expr::FieldAccess(access) = call.callee() else {
            continue;
        };
        if access.field().get().as_str() != "with" {
            continue;
        }
        let ast::Expr::Ident(id) = access.target() else {
            continue;
        };
        let kind = match id.get().as_str() {
            "elsearticle" => TemplateKind::Elsarticle,
            "ieee" => TemplateKind::Ieee,
            _ => continue,
        };
        found = Some((kind, call.args()));
        break;
    }

    let (kind, args) = found?;

    // Evaluate the document's top-level `#let` bindings so argument expressions
    // such as `abstract: abstract` resolve.
    let mut eval = MiniEval::new();
    for expr in markup.exprs() {
        if matches!(expr, ast::Expr::LetBinding(_)) {
            let _ = eval.eval_expr(expr);
        }
    }

    let mut title = String::new();
    let mut abstract_text = String::new();
    let mut keywords = Vec::new();
    let mut authors = Vec::new();
    let mut affiliations = Vec::new();
    let mut journal = None;

    for arg in args.items() {
        let ast::Arg::Named(named) = arg else { continue };
        let Ok(value) = eval.eval_expr(named.expr()) else {
            continue;
        };
        match named.name().get().as_str() {
            "title" => title = value_to_inline_latex(&value),
            "abstract" => abstract_text = value_to_block_latex(&value),
            "keywords" | "index-terms" => keywords = value_to_string_list(&value),
            "authors" => authors = parse_authors(&value),
            "affiliations" => affiliations = parse_affiliations(&value),
            "journal" => {
                let j = value_to_inline_latex(&value);
                if !j.is_empty() {
                    journal = Some(j);
                }
            }
            _ => {}
        }
    }

    Some(Frontmatter {
        kind,
        title,
        authors,
        affiliations,
        abstract_text,
        keywords,
        journal,
    })
}

fn parse_authors(value: &Value) -> Vec<Author> {
    let Value::Array(items) = value else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let Value::Dict(dict) = item else {
                return None;
            };
            Some(Author {
                name: dict.get("name").map(value_to_inline_latex).unwrap_or_default(),
                email: dict
                    .get("email")
                    .and_then(|v| v.as_str().ok())
                    .map(|s| s.to_string()),
                corresponding: matches!(dict.get("corresponding"), Some(Value::Bool(true))),
                affiliations: dict
                    .get("affiliations")
                    .map(value_to_string_list)
                    .unwrap_or_default(),
                organization: dict.get("organization").map(value_to_inline_latex),
                location: dict.get("location").map(value_to_inline_latex),
            })
        })
        .collect()
}

fn parse_affiliations(value: &Value) -> Vec<(String, String)> {
    let Value::Dict(dict) = value else {
        return Vec::new();
    };
    dict.iter()
        .map(|(id, v)| (id.clone(), value_to_inline_latex(v)))
        .collect()
}

fn value_to_string_list(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items.iter().map(value_to_inline_latex).collect(),
        Value::Str(s) => vec![s.clone()],
        _ => Vec::new(),
    }
}

/// Render a value (string or content) to a single-line LaTeX fragment.
fn value_to_inline_latex(value: &Value) -> String {
    match value {
        Value::Str(s) => s.clone(),
        Value::Content(nodes) => content_to_latex(nodes),
        other => other.display(),
    }
}

/// Render a value to a LaTeX block (used for the abstract).
fn value_to_block_latex(value: &Value) -> String {
    value_to_inline_latex(value).trim().to_string()
}

fn content_to_latex(nodes: &[ContentNode]) -> String {
    let mut ctx = ConvertContext::new();
    convert_content_nodes_to_latex(nodes, &mut ctx);
    ctx.finalize().trim().to_string()
}

/// Wrap the converted `body` in a complete LaTeX document for the detected
/// template. Mirrors the slicing the hand-written build scripts did: the body is
/// taken from its first sectioning command, and any in-body bibliography call is
/// re-emitted with the template's citation style just before `\end{document}`.
pub fn assemble_document(fm: &Frontmatter, body: &str) -> String {
    let bib = extract_bibliography_name(body).unwrap_or_else(|| "references".to_string());
    let body = slice_body(body);

    let mut doc = render_preamble(fm);
    doc.push_str(body.trim());
    doc.push_str("\n\n");
    doc.push_str(&render_closing(fm, &bib));
    doc
}

/// Take the body from its first sectioning command onward, dropping any leaked
/// template imports/front matter, and stop before an in-body bibliography.
fn slice_body(body: &str) -> &str {
    const HEADS: [&str; 4] = ["\\section{", "\\section*{", "\\part{", "\\chapter{"];
    let start = HEADS
        .iter()
        .filter_map(|h| body.find(h))
        .min()
        .unwrap_or(0);
    let sliced = &body[start..];
    match sliced.find("\\bibliographystyle") {
        Some(end) => sliced[..end].trim_end(),
        None => sliced.trim_end(),
    }
}

fn extract_bibliography_name(body: &str) -> Option<String> {
    let idx = body.find("\\bibliography{")?;
    let rest = &body[idx + "\\bibliography{".len()..];
    let end = rest.find('}')?;
    Some(rest[..end].to_string())
}

fn render_preamble(fm: &Frontmatter) -> String {
    match fm.kind {
        TemplateKind::Elsarticle => render_elsarticle_preamble(fm),
        TemplateKind::Ieee => render_ieee_preamble(fm),
    }
}

fn render_closing(fm: &Frontmatter, bib: &str) -> String {
    let style = match fm.kind {
        TemplateKind::Elsarticle => "elsarticle-harv",
        TemplateKind::Ieee => "IEEEtran",
    };
    format!("\\bibliographystyle{{{style}}}\n\\bibliography{{{bib}}}\n\n\\end{{document}}\n")
}

fn render_elsarticle_preamble(fm: &Frontmatter) -> String {
    let mut s = String::new();
    s.push_str(
        "\\documentclass[review,12pt]{elsarticle}\n\
\\usepackage{amsmath,amssymb,amsfonts}\n\
\\usepackage{graphicx}\n\
\\usepackage{textcomp}\n\
\\usepackage{xcolor}\n\
\\usepackage{booktabs}\n\
\\usepackage{algorithm}\n\
\\usepackage{algpseudocode}\n\
\\usepackage{lineno}\n\
\\biboptions{authoryear}\n\
\\let\\cite\\citep\n\
\\setlength{\\emergencystretch}{2em}\n\
\\hfuzz=15pt\n\
\\hbadness=10000\n",
    );
    // `\journal{...}` drives the "Preprint submitted to <journal>" footer.
    if let Some(journal) = &fm.journal {
        s.push_str(&format!("\\journal{{{journal}}}\n"));
    }
    s.push_str("\n\\begin{document}\n\n\\begin{frontmatter}\n\n");
    s.push_str(&format!("\\title{{{}}}\n\n", fm.title));

    let mut cor_emitted = false;
    for author in &fm.authors {
        let aff = if author.affiliations.is_empty() {
            String::new()
        } else {
            format!("[{}]", author.affiliations.join(","))
        };
        let cor = if author.corresponding {
            cor_emitted = true;
            "\\corref{cor1}"
        } else {
            ""
        };
        s.push_str(&format!("\\author{aff}{{{}{cor}}}\n", author.name));
        if let Some(email) = &author.email {
            s.push_str(&format!("\\ead{{{email}}}\n"));
        }
        s.push('\n');
    }

    for (id, text) in &fm.affiliations {
        let (org, country) = split_org_country(text);
        match country {
            Some(country) => s.push_str(&format!(
                "\\affiliation[{id}]{{organization={{{org}}}, country={{{country}}}}}\n"
            )),
            None => s.push_str(&format!("\\affiliation[{id}]{{organization={{{org}}}}}\n")),
        }
    }
    if !fm.affiliations.is_empty() {
        s.push('\n');
    }

    if cor_emitted {
        s.push_str("\\cortext[cor1]{Corresponding author}\n\n");
    }

    s.push_str(&format!(
        "\\begin{{abstract}}\n{}\n\\end{{abstract}}\n\n",
        fm.abstract_text
    ));

    if !fm.keywords.is_empty() {
        s.push_str(&format!(
            "\\begin{{keyword}}\n{}\n\\end{{keyword}}\n\n",
            fm.keywords.join(" \\sep ")
        ));
    }

    s.push_str("\\end{frontmatter}\n\n\\linenumbers\n\n");
    s
}

fn render_ieee_preamble(fm: &Frontmatter) -> String {
    let mut s = String::new();
    s.push_str(
        "\\documentclass[conference]{IEEEtran}\n\
\\usepackage{amsmath,amssymb,amsfonts}\n\
\\usepackage{graphicx}\n\
\\usepackage{textcomp}\n\
\\usepackage{xcolor}\n\
\\usepackage{booktabs}\n\
\\usepackage{array}\n\
\\usepackage{algorithm}\n\
\\usepackage{algpseudocode}\n\
\\setlength{\\tabcolsep}{4pt}\n\
\\renewcommand{\\arraystretch}{1.18}\n\
\\setlength{\\emergencystretch}{2em}\n\
\\hfuzz=15pt\n\
\\hbadness=10000\n\n\
\\begin{document}\n\n",
    );
    s.push_str(&format!("\\title{{{}}}\n\n", fm.title));

    s.push_str("\\author{\n");
    for (i, author) in fm.authors.iter().enumerate() {
        if i > 0 {
            s.push_str("\\and\n");
        }
        s.push_str(&format!("\\IEEEauthorblockN{{{}}}\n", author.name));
        let mut lines = Vec::new();
        if let Some(org) = &author.organization {
            lines.push(org.clone());
        }
        if let Some(loc) = &author.location {
            lines.push(loc.clone());
        }
        if let Some(email) = &author.email {
            lines.push(email.clone());
        }
        s.push_str(&format!(
            "\\IEEEauthorblockA{{{}}}\n",
            lines.join("\\\\\n")
        ));
    }
    s.push_str("}\n\n\\maketitle\n\n");

    s.push_str(&format!(
        "\\begin{{abstract}}\n{}\n\\end{{abstract}}\n\n",
        fm.abstract_text
    ));

    if !fm.keywords.is_empty() {
        s.push_str(&format!(
            "\\begin{{IEEEkeywords}}\n{}\n\\end{{IEEEkeywords}}\n\n",
            fm.keywords.join(", ")
        ));
    }
    s
}

/// elsarticle wants `organization=` and `country=` separately; the Typst
/// affiliation is a single "Organization, Country" string. Split on the last
/// comma.
fn split_org_country(text: &str) -> (String, Option<String>) {
    match text.rsplit_once(',') {
        Some((org, country)) => (org.trim().to_string(), Some(country.trim().to_string())),
        None => (text.trim().to_string(), None),
    }
}
