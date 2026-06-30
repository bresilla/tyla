//! Detection and emission of Typst paper-template show rules from LaTeX.
//!
//! This is the reverse of `typst2latex::template`. When a LaTeX document uses
//! the `elsarticle` or `IEEEtran` class, its front matter (title, authors,
//! affiliations, abstract, keywords) is parsed here and re-emitted as the
//! corresponding Typst template call:
//!
//! ```typst
//! #import "@preview/elsearticle:3.1.0": *
//! #show: elsearticle.with(title: ..., authors: (...), ...)
//! ```
//!
//! so that a round-trip preserves the template instead of degrading to a generic
//! document.

use super::context::LatexConverter;
use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateKind {
    Elsarticle,
    Ieee,
}

#[derive(Debug, Clone, Default)]
struct Author {
    name: String,
    email: Option<String>,
    corresponding: bool,
    affiliations: Vec<String>,
    organization: Option<String>,
    location: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Frontmatter {
    kind: TemplateKind,
    title: String,
    authors: Vec<Author>,
    affiliations: Vec<(String, String)>,
    abstract_text: String,
    keywords: Vec<String>,
    /// elsarticle target journal (drives the "submitted to …" footer).
    journal: Option<String>,
    /// Document uses lovelace algorithms, so the package must be imported.
    needs_lovelace: bool,
}

/// Read the balanced `{...}` group starting at the first `{` in `s`, returning
/// the inner text and the byte offset just past the closing brace.
fn read_balanced(s: &str) -> Option<(String, usize)> {
    let open = s.find('{')?;
    let region = &s[open..];
    let mut depth = 0usize;
    for (i, ch) in region.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((region[1..i].to_string(), open + i + 1));
                }
            }
            _ => {}
        }
    }
    None
}

/// Remove LaTeX line comments (`%` to end of line, respecting `\%`). Keeps line
/// breaks so positions and `\begin{...}` matching are unaffected.
fn strip_latex_comments(src: &str) -> String {
    src.lines()
        .map(|line| {
            let bytes = line.as_bytes();
            let mut i = 0;
            let mut cut = line.len();
            while i < bytes.len() {
                match bytes[i] {
                    b'\\' => i += 2, // skip an escaped character (e.g. \%)
                    b'%' => {
                        cut = i;
                        break;
                    }
                    _ => i += 1,
                }
            }
            &line[..cut.min(line.len())]
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn braced_after<'a>(text: &'a str, marker: &str) -> Option<String> {
    let idx = text.find(marker)?;
    read_balanced(&text[idx + marker.len()..]).map(|(inner, _)| inner)
}

fn environment_body(text: &str, name: &str) -> Option<String> {
    let begin = format!("\\begin{{{name}}}");
    let end = format!("\\end{{{name}}}");
    let b = text.find(&begin)? + begin.len();
    let e = text.find(&end)?;
    if e <= b {
        return None;
    }
    Some(text[b..e].to_string())
}

/// Detect an `elsarticle`/`IEEEtran` document and extract its front matter,
/// converting LaTeX fragments to Typst eagerly (needs `&mut conv`).
pub fn detect_frontmatter(conv: &mut LatexConverter, raw: &str) -> Option<Frontmatter> {
    let class_re = Regex::new(r"\\documentclass(?:\[[^\]]*\])?\{([^}]+)\}").ok()?;
    let class = class_re.captures(raw)?.get(1)?.as_str().trim().to_string();
    let kind = match class.as_str() {
        "elsarticle" => TemplateKind::Elsarticle,
        "IEEEtran" => TemplateKind::Ieee,
        _ => return None,
    };

    // Parse from a comment-stripped copy so commented-out example authors,
    // affiliations, and `%% keywords` lines do not leak into the front matter.
    let src = &strip_latex_comments(raw);

    let title = braced_after(src, "\\title")
        .map(|t| conv.convert_fragment(&t))
        .unwrap_or_default();
    let abstract_text = environment_body(src, "abstract")
        .map(|a| conv.convert_fragment(a.trim()))
        .unwrap_or_default();

    let (authors, affiliations, keywords) = match kind {
        TemplateKind::Elsarticle => parse_elsarticle(conv, src),
        TemplateKind::Ieee => parse_ieee(conv, src),
    };

    let journal = braced_after(src, "\\journal").map(|j| conv.convert_fragment(&j));

    Some(Frontmatter {
        kind,
        title,
        authors,
        affiliations,
        abstract_text,
        keywords,
        journal,
        needs_lovelace: src.contains("\\begin{algorithm}"),
    })
}

/// The lovelace import and algorithm caption setup, emitted when the document
/// contains algorithms.
fn lovelace_preamble(needs_lovelace: bool) -> String {
    if needs_lovelace {
        "#import \"@preview/lovelace:0.3.0\": *\n\
         #show figure.where(kind: \"algorithm\"): set figure.caption(position: top)\n\n"
            .to_string()
    } else {
        String::new()
    }
}

fn parse_elsarticle(
    conv: &mut LatexConverter,
    src: &str,
) -> (Vec<Author>, Vec<(String, String)>, Vec<String>) {
    let mut authors = Vec::new();
    // \author[aff1,aff2]{Name\corref{cor1}}  optionally followed by \ead{email}
    let author_re =
        Regex::new(r"\\author(?:\[([^\]]*)\])?\{").expect("author regex");
    let mut search_from = 0usize;
    while let Some(m) = author_re.find_at(src, search_from) {
        let affs_raw = author_re
            .captures(&src[m.start()..])
            .and_then(|c| c.get(1).map(|g| g.as_str().to_string()))
            .unwrap_or_default();
        let brace_start = m.end() - 1; // position of '{'
        let Some((name_raw, consumed)) = read_balanced(&src[brace_start..]) else {
            break;
        };
        let after = brace_start + consumed;
        let corresponding = name_raw.contains("\\corref");
        // Strip \corref{...} / \thanksref{...} from the displayed name.
        let name_clean = Regex::new(r"\\(corref|thanksref|fnref)\{[^}]*\}")
            .unwrap()
            .replace_all(&name_raw, "")
            .to_string();
        let name = conv.convert_fragment(name_clean.trim());

        // Look for a \ead before the next \author.
        let tail = &src[after..];
        let next_author = author_re.find(tail).map(|mm| mm.start()).unwrap_or(tail.len());
        let email = braced_after(&tail[..next_author], "\\ead");

        search_from = after;
        if name.trim().is_empty() {
            continue; // empty placeholder author
        }

        let affiliations = affs_raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        authors.push(Author {
            name,
            email,
            corresponding,
            affiliations,
            organization: None,
            location: None,
        });
    }

    // \affiliation[id]{organization={ORG}, country={COUNTRY}}  (or plain text)
    let mut affiliations = Vec::new();
    let aff_re = Regex::new(r"\\affiliation(?:\[([^\]]*)\])?\{").expect("aff regex");
    let mut from = 0usize;
    while let Some(m) = aff_re.find_at(src, from) {
        let id = aff_re
            .captures(&src[m.start()..])
            .and_then(|c| c.get(1).map(|g| g.as_str().trim().to_string()))
            .unwrap_or_default();
        let brace = m.end() - 1;
        let Some((body, consumed)) = read_balanced(&src[brace..]) else {
            break;
        };
        from = brace + consumed;
        let text = elsarticle_affiliation_text(conv, &body);
        if !id.is_empty() {
            affiliations.push((id, text));
        }
    }

    let keywords = environment_body(src, "keyword")
        .map(|k| split_keywords(&k, r"\sep"))
        .unwrap_or_default();

    (authors, affiliations, keywords)
}

fn elsarticle_affiliation_text(conv: &mut LatexConverter, body: &str) -> String {
    // organization={X}, country={Y} -> "X, Y"; otherwise convert as-is.
    let org = braced_after(body, "organization=");
    let country = braced_after(body, "country=");
    match (org, country) {
        (Some(o), Some(c)) => format!("{}, {}", conv.convert_fragment(&o), conv.convert_fragment(&c)),
        (Some(o), None) => conv.convert_fragment(&o),
        _ => conv.convert_fragment(body.trim()),
    }
}

fn parse_ieee(
    conv: &mut LatexConverter,
    src: &str,
) -> (Vec<Author>, Vec<(String, String)>, Vec<String>) {
    let mut authors = Vec::new();
    if let Some(block) = braced_after(src, "\\author") {
        for chunk in block.split("\\and") {
            let Some(name) = braced_after(chunk, "\\IEEEauthorblockN") else {
                continue;
            };
            let name = conv.convert_fragment(name.trim());
            if name.is_empty() {
                continue;
            }
            let info = braced_after(chunk, "\\IEEEauthorblockA").unwrap_or_default();
            let mut parts = info
                .split("\\\\")
                .map(|p| p.trim())
                .filter(|p| !p.is_empty());
            let organization = parts.next().map(|p| conv.convert_fragment(p));
            let location = parts.next().map(|p| conv.convert_fragment(p));
            let email = parts.next().map(|p| p.to_string());
            authors.push(Author {
                name,
                email,
                corresponding: false,
                affiliations: Vec::new(),
                organization,
                location,
            });
        }
    }

    let keywords = environment_body(src, "IEEEkeywords")
        .map(|k| split_keywords(&k, ","))
        .unwrap_or_default();

    (authors, Vec::new(), keywords)
}

fn split_keywords(body: &str, sep: &str) -> Vec<String> {
    body.split(sep)
        .map(|k| k.trim().trim_end_matches('.').trim().to_string())
        .filter(|k| !k.is_empty())
        .collect()
}

impl Frontmatter {
    /// Render the `#import` + `#show: …with(...)` Typst show rule.
    pub fn render_show_rule(&self) -> String {
        match self.kind {
            TemplateKind::Elsarticle => self.render_elsarticle(),
            TemplateKind::Ieee => self.render_ieee(),
        }
    }

    fn render_elsarticle(&self) -> String {
        let mut s = String::from("#import \"@preview/elsearticle:3.1.0\": *\n");
        s.push_str(&lovelace_preamble(self.needs_lovelace));
        s.push_str("#show: elsearticle.with(\n");
        s.push_str(&format!("  title: [{}],\n", self.title));
        s.push_str("  authors: (\n");
        for a in &self.authors {
            let affs = a
                .affiliations
                .iter()
                .map(|x| format!("\"{x}\""))
                .collect::<Vec<_>>()
                .join(", ");
            let affs = if a.affiliations.len() == 1 {
                format!("({affs},)")
            } else {
                format!("({affs})")
            };
            s.push_str(&format!("    (name: [{}], affiliations: {affs}", a.name));
            if a.corresponding {
                s.push_str(", corresponding: true");
            }
            if let Some(email) = &a.email {
                s.push_str(&format!(", email: \"{email}\""));
            }
            s.push_str("),\n");
        }
        s.push_str("  ),\n");
        s.push_str("  affiliations: (\n");
        for (id, text) in &self.affiliations {
            s.push_str(&format!("    \"{id}\": [{text}],\n"));
        }
        s.push_str("  ),\n");
        if let Some(journal) = &self.journal {
            s.push_str(&format!("  journal: \"{journal}\",\n"));
        }
        s.push_str(&format!("  abstract: [{}],\n", self.abstract_text));
        s.push_str(&format!("  keywords: ({}),\n", quoted_list(&self.keywords)));
        s.push_str("  format: \"preprint\",\n");
        s.push_str(")\n\n");
        s
    }

    fn render_ieee(&self) -> String {
        let mut s = String::from("#import \"@preview/charged-ieee:0.1.3\": ieee\n");
        s.push_str(&lovelace_preamble(self.needs_lovelace));
        s.push_str("#show: ieee.with(\n");
        s.push_str(&format!("  title: [{}],\n", self.title));
        s.push_str("  authors: (\n");
        for a in &self.authors {
            s.push_str(&format!("    (\n      name: \"{}\",\n", strip_brackets(&a.name)));
            if let Some(org) = &a.organization {
                s.push_str(&format!("      organization: [{org}],\n"));
            }
            if let Some(loc) = &a.location {
                s.push_str(&format!("      location: [{loc}],\n"));
            }
            if let Some(email) = &a.email {
                s.push_str(&format!("      email: \"{email}\",\n"));
            }
            s.push_str("    ),\n");
        }
        s.push_str("  ),\n");
        s.push_str(&format!("  abstract: [{}],\n", self.abstract_text));
        s.push_str(&format!("  index-terms: ({}),\n", quoted_list(&self.keywords)));
        s.push_str(")\n\n");
        s
    }
}

fn quoted_list(items: &[String]) -> String {
    items
        .iter()
        .map(|i| format!("\"{i}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

fn strip_brackets(s: &str) -> String {
    s.trim().trim_start_matches('[').trim_end_matches(']').trim().to_string()
}
