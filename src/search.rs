//! Build-time search index. Small on purpose: per page section (split at headings)
//! it keeps the heading and the *set* of lowercase words, no sentences, no code.
//! The browser does prefix matching on it (see common.js); the index is only
//! fetched when someone uses the search box.

use std::collections::HashSet;

const STOP: &[&str] = &[
    "the", "and", "for", "are", "but", "not", "you", "all", "can", "was", "one", "our", "out",
    "has", "have", "had", "with", "this", "that", "from", "they", "will", "would", "there",
    "their", "what", "about", "which", "when", "your", "been", "more", "also", "into", "than",
    "then", "them", "these", "those", "such", "each", "other", "some", "any", "may", "its", "is",
    "it", "in", "on", "of", "to", "as", "at", "be", "by", "do", "if", "or", "so", "we", "an", "he",
    "she", "his", "her", "who", "how", "why", "use", "used", "using", "just", "only", "very",
    "does", "did", "should", "could", "were", "while", "where", "here",
];

struct Section {
    page: usize,
    anchor: String,
    heading: String,
    words: String,
}

#[derive(Default)]
pub struct SearchIndex {
    /// (title, target): the output path (static) or section id (single file).
    pages: Vec<(String, String)>,
    sections: Vec<Section>,
}

fn decode(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' | '\r' | '\t' => o.push(' '),
            c if (c as u32) < 0x20 => {}
            c => o.push(c),
        }
    }
    o + "\""
}

/// Distinct searchable words of `text`, in order of first appearance.
fn words(text: &str, seen: &mut HashSet<String>, out: &mut Vec<String>) {
    for w in text.split(|c: char| !c.is_alphanumeric()) {
        let w = w.to_lowercase();
        let n = w.chars().count();
        if n < 2 || n > 32 || STOP.contains(&w.as_str()) {
            continue;
        }
        if seen.insert(w.clone()) {
            out.push(w);
        }
    }
}

/// Value of `name="..."` in a tag's source.
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let i = tag.find(&key)? + key.len();
    let rest = &tag[i..];
    Some(&rest[..rest.find('"')?])
}

impl SearchIndex {
    /// Adds a rendered page. `target` identifies it for the browser; `id_prefix` is
    /// stripped from heading ids (single-file output prefixes them per page).
    pub fn add_page(&mut self, title: &str, target: &str, html: &str, id_prefix: &str) {
        let page = self.pages.len();
        self.pages.push((title.to_string(), target.to_string()));
        let mut cur = (String::new(), title.to_string()); // (anchor, heading)
        let mut seen = HashSet::new();
        let mut list = Vec::new();
        let mut in_heading = false;
        let mut heading_text = String::new();
        let mut skip: Option<&str> = None;
        let flush = |cur: &(String, String),
                     seen: &mut HashSet<String>,
                     list: &mut Vec<String>,
                     sections: &mut Vec<Section>| {
            if !list.is_empty() {
                sections.push(Section {
                    page,
                    anchor: cur.0.clone(),
                    heading: cur.1.clone(),
                    words: list.join(" "),
                });
            }
            seen.clear();
            list.clear();
        };

        let mut rest = html;
        while !rest.is_empty() {
            let Some(lt) = rest.find('<') else {
                if skip.is_none() {
                    let t = decode(rest);
                    if in_heading {
                        heading_text.push_str(&t);
                    }
                    words(&t, &mut seen, &mut list);
                }
                break;
            };
            if skip.is_none() {
                let t = decode(&rest[..lt]);
                if in_heading {
                    heading_text.push_str(&t);
                }
                words(&t, &mut seen, &mut list);
            }
            rest = &rest[lt..];
            let Some(gt) = rest.find('>') else { break };
            let tag = &rest[1..gt];
            rest = &rest[gt + 1..];
            let closing = tag.starts_with('/');
            let name: String = tag
                .trim_start_matches('/')
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_lowercase();
            if let Some(s) = skip {
                if closing && name == s {
                    skip = None;
                }
                continue;
            }
            match (name.as_str(), closing) {
                ("pre" | "script" | "style" | "svg", false) => {
                    skip = Some(match name.as_str() {
                        "pre" => "pre",
                        "script" => "script",
                        "style" => "style",
                        _ => "svg",
                    });
                }
                ("h1" | "h2" | "h3" | "h4" | "h5" | "h6", false) => {
                    flush(&cur, &mut seen, &mut list, &mut self.sections);
                    let id = attr(tag, "id").unwrap_or("");
                    cur = (
                        id.strip_prefix(id_prefix).unwrap_or(id).to_string(),
                        String::new(),
                    );
                    in_heading = true;
                    heading_text.clear();
                }
                ("h1" | "h2" | "h3" | "h4" | "h5" | "h6", true) => {
                    in_heading = false;
                    let h = heading_text
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    cur.1 = if h.is_empty() { title.to_string() } else { h };
                    // The heading's own words belong to its section.
                    words(&cur.1.clone(), &mut seen, &mut list);
                }
                _ => {}
            }
        }
        flush(&cur, &mut seen, &mut list, &mut self.sections);
    }

    /// `{"p":[[title,target],...],"s":[[page,anchor,heading,"words"],...]}`
    pub fn to_json(&self) -> String {
        let pages: Vec<String> = self
            .pages
            .iter()
            .map(|(t, u)| format!("[{},{}]", json_str(t), json_str(u)))
            .collect();
        let sections: Vec<String> = self
            .sections
            .iter()
            .map(|s| {
                format!(
                    "[{},{},{},{}]",
                    s.page,
                    json_str(&s.anchor),
                    json_str(&s.heading),
                    json_str(&s.words)
                )
            })
            .collect();
        format!(
            "{{\"p\":[{}],\"s\":[{}]}}",
            pages.join(","),
            sections.join(",")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_sections_and_skips_code() {
        let mut ix = SearchIndex::default();
        ix.add_page(
            "Guide",
            "guide.html",
            "<h1 id=\"guide\">Guide</h1><p>Install the tool. Install again.</p>\
             <pre><code>secretcode</code></pre><h2 id=\"p--setup\">Set up</h2><p>Configure &amp; run.</p>",
            "p--",
        );
        assert_eq!(ix.sections.len(), 2);
        assert_eq!(ix.sections[0].anchor, "guide");
        assert_eq!(ix.sections[0].words, "guide install tool again");
        assert_eq!(ix.sections[1].anchor, "setup");
        assert_eq!(ix.sections[1].heading, "Set up");
        assert!(ix.sections[1].words.contains("configure"));
        assert!(!ix.to_json().contains("secretcode"));
    }
}
