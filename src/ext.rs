//! Markdown extensions that comrak doesn't know, done as a source-to-source pass
//! before parsing. The syntax follows Python-Markdown / pymdownx, so existing
//! MkDocs and Zensical content works:
//!
//! - content tabs:   `=== "Title"` plus an indented body
//! - admonitions:    `!!! note "Title"` plus an indented body
//! - collapsible:    `??? note "Title"` (`???+` starts open)
//! - snippets:       `--8<-- "path/to/file"` includes another file
//!
//! Bodies are indented four spaces more than their marker line. Fenced code blocks
//! are passed through untouched.

use std::path::Path;

const MAX_DEPTH: u8 = 5;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn indent_of(line: &str) -> usize {
    let mut n = 0;
    for c in line.chars() {
        match c {
            ' ' => n += 1,
            '\t' => n += 4,
            _ => break,
        }
    }
    n
}

/// Removes up to `n` columns of leading whitespace.
fn dedent(line: &str, n: usize) -> String {
    let mut cols = 0;
    for (i, c) in line.char_indices() {
        if cols >= n {
            return line[i..].to_string();
        }
        match c {
            ' ' => cols += 1,
            '\t' => cols += 4,
            _ => return line[i..].to_string(),
        }
    }
    String::new()
}

/// `"Title"` at the start of `s`: the title and the rest.
fn quoted(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start().strip_prefix('"')?;
    let end = s.find('"')?;
    Some((&s[..end], &s[end + 1..]))
}

enum Marker {
    Tab(String),
    /// (kind, collapsible, open, title)
    Admonition(String, bool, bool, String),
}

fn marker(line: &str) -> Option<Marker> {
    let t = line.trim();
    if let Some(rest) = t.strip_prefix("===") {
        if !rest.starts_with(' ') {
            return None;
        }
        let (title, tail) = quoted(rest)?;
        return tail
            .trim()
            .is_empty()
            .then(|| Marker::Tab(title.to_string()));
    }
    let (collapsible, open, rest) = if let Some(r) = t.strip_prefix("???+") {
        (true, true, r)
    } else if let Some(r) = t.strip_prefix("???") {
        (true, false, r)
    } else {
        let r = t.strip_prefix("!!!")?;
        (false, false, r)
    };
    if !rest.starts_with(' ') {
        return None;
    }
    let rest = rest.trim_start();
    let kind: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if kind.is_empty() {
        return None;
    }
    let tail = rest[kind.len()..].trim();
    let title = if tail.is_empty() {
        let mut c = kind.chars();
        let first = c.next().unwrap().to_uppercase().collect::<String>();
        first + c.as_str()
    } else {
        let (t, after) = quoted(tail)?;
        if !after.trim().is_empty() {
            return None;
        }
        t.to_string()
    };
    Some(Marker::Admonition(
        kind.to_lowercase(),
        collapsible,
        open,
        title,
    ))
}

/// Maps the many admonition names onto the five colours the stylesheet has.
fn admonition_class(kind: &str) -> &'static str {
    match kind {
        "tip" | "hint" | "success" | "check" | "done" => "tip",
        "important" | "question" | "help" | "faq" => "important",
        "warning" | "caution-note" | "attention" => "warning",
        "danger" | "error" | "failure" | "fail" | "missing" | "bug" | "caution" => "caution",
        _ => "note",
    }
}

/// The body of a block starting after line `i`: lines indented past `indent`
/// (blank lines inside are kept). Returns the dedented body and the next index.
fn body(lines: &[String], i: usize, indent: usize) -> (Vec<String>, usize) {
    let mut j = i;
    while j < lines.len() && (lines[j].trim().is_empty() || indent_of(&lines[j]) >= indent + 4) {
        j += 1;
    }
    let mut end = j;
    while end > i && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    let b = lines[i..end]
        .iter()
        .map(|l| dedent(l, indent + 4))
        .collect();
    (b, end)
}

fn process(lines: &[String], base: &Path, root: &Path, depth: u8) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        let t = line.trim_start();
        if let Some((c, n)) = fence {
            let run = t.chars().take_while(|x| *x == c).count();
            if run >= n && t[run..].trim().is_empty() {
                fence = None;
            }
            out.push(line.clone());
            i += 1;
            continue;
        }
        if let Some(c) = t.chars().next().filter(|c| *c == '`' || *c == '~') {
            let run = t.chars().take_while(|x| *x == c).count();
            if run >= 3 {
                fence = Some((c, run));
                out.push(line.clone());
                i += 1;
                continue;
            }
        }
        let pad = " ".repeat(indent_of(line));
        if let Some(rest) = t.strip_prefix("--8<--")
            && let Some((path, _)) = quoted(rest)
        {
            let path = path.split(':').next().unwrap_or(path);
            if depth < MAX_DEPTH
                && let Some(text) = [base, root]
                    .iter()
                    .map(|d| d.join(path))
                    .find(|p| p.is_file())
                    .and_then(|p| std::fs::read_to_string(p).ok())
            {
                let inner: Vec<String> = text.lines().map(String::from).collect();
                for l in process(&inner, base, root, depth + 1) {
                    out.push(if l.is_empty() { l } else { format!("{pad}{l}") });
                }
                i += 1;
                continue;
            }
        }
        let indent = indent_of(line);
        match marker(line) {
            Some(Marker::Tab(_)) => {
                // A run of tabs at the same indent forms one group.
                out.push(format!("{pad}<div class=\"tabbed\">"));
                out.push(String::new());
                while i < lines.len() {
                    let Some(Marker::Tab(title)) = marker(&lines[i]) else {
                        break;
                    };
                    if indent_of(&lines[i]) != indent {
                        break;
                    }
                    let (b, next) = body(lines, i + 1, indent);
                    out.push(format!(
                        "{pad}<div class=\"tab\" data-title=\"{}\">",
                        esc(&title)
                    ));
                    out.push(String::new());
                    for l in process(&b, base, root, depth) {
                        out.push(if l.is_empty() { l } else { format!("{pad}{l}") });
                    }
                    out.push(String::new());
                    out.push(format!("{pad}</div>"));
                    out.push(String::new());
                    i = next;
                    // Another tab may follow after blank lines.
                    let mut k = i;
                    while k < lines.len() && lines[k].trim().is_empty() {
                        k += 1;
                    }
                    if k < lines.len()
                        && matches!(marker(&lines[k]), Some(Marker::Tab(_)))
                        && indent_of(&lines[k]) == indent
                    {
                        i = k;
                    } else {
                        break;
                    }
                }
                out.push(format!("{pad}</div>"));
                out.push(String::new());
            }
            Some(Marker::Admonition(kind, collapsible, open, title)) => {
                let (b, next) = body(lines, i + 1, indent);
                let class = admonition_class(&kind);
                let kind = esc(&kind);
                if collapsible {
                    out.push(format!(
                        "{pad}<details class=\"admonition {class} {kind}\"{}>",
                        if open { " open" } else { "" }
                    ));
                    out.push(format!("{pad}<summary>{}</summary>", esc(&title)));
                } else {
                    out.push(format!("{pad}<div class=\"admonition {class} {kind}\">"));
                    if !title.is_empty() {
                        out.push(format!(
                            "{pad}<p class=\"admonition-title\">{}</p>",
                            esc(&title)
                        ));
                    }
                }
                out.push(String::new());
                for l in process(&b, base, root, depth) {
                    out.push(if l.is_empty() { l } else { format!("{pad}{l}") });
                }
                out.push(String::new());
                out.push(format!(
                    "{pad}</{}>",
                    if collapsible { "details" } else { "div" }
                ));
                out.push(String::new());
                i = next;
                continue;
            }
            None => {
                out.push(line.clone());
            }
        }
        if !matches!(marker(line), Some(Marker::Tab(_))) {
            i += 1;
        }
    }
    out
}

/// Expands the extensions in `md`. `base` is the folder of the page (for snippet
/// paths); `root` is the site folder.
pub fn preprocess(md: &str, base: &Path, root: &Path) -> String {
    // Cheap exit for pages that use none of this.
    if !(md.contains("===") || md.contains("!!!") || md.contains("???") || md.contains("--8<--")) {
        return md.to_string();
    }
    let lines: Vec<String> = md.lines().map(String::from).collect();
    let mut s = process(&lines, base, root, 0).join("\n");
    if md.ends_with('\n') {
        s.push('\n');
    }
    s
}
