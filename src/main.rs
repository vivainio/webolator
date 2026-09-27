//! mdsite: turn a markdown file or a folder of markdown into a browsable site.
//! No config, no index files. Mermaid is rendered client-side.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use base64::Engine;
use clap::Parser;
use comrak::nodes::{AstNode, NodeHtmlBlock, NodeValue};
use comrak::options::Plugins;
use comrak::plugins::syntect::{SyntectAdapter, SyntectAdapterBuilder};
use comrak::{Arena, Options, format_html_with_plugins, parse_document};
use std::sync::OnceLock;
use walkdir::WalkDir;

const MERMAID_CDN: &str = "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js";
const STYLE: &str = include_str!("style.css");
const SINGLE_JS: &str = include_str!("single.js");

const TEXT_EXT: &[&str] = &[
    "txt",
    "text",
    "log",
    "csv",
    "tsv",
    "json",
    "yaml",
    "yml",
    "toml",
    "xml",
    "ini",
    "cfg",
    "conf",
    "sh",
    "bash",
    "zsh",
    "ps1",
    "bat",
    "py",
    "rs",
    "js",
    "ts",
    "go",
    "java",
    "kt",
    "c",
    "h",
    "cpp",
    "hpp",
    "cs",
    "rb",
    "sql",
    "diff",
    "patch",
    "env",
    "properties",
];
/// Written into static output dirs so later runs don't treat them as content.
const OUTPUT_MARKER: &str = ".mdsite-output";
const SKIP_DIRS: &[&str] = &["node_modules", "target", "__pycache__", "venv"];

#[derive(Parser)]
#[command(about = "Render markdown files into a zero-config website (with mermaid)")]
struct Cli {
    /// Markdown file or directory
    input: PathBuf,
    /// Output directory (default ./site), or output file with --single (default <name>.html)
    #[arg(short, long)]
    out: Option<PathBuf>,
    /// Produce one self-contained HTML file
    #[arg(long)]
    single: bool,
    /// Serve locally with live reload instead of writing output
    #[arg(long)]
    serve: bool,
    #[arg(long, default_value_t = 8000)]
    port: u16,
    /// Use this local mermaid.min.js (inlined/copied) instead of the CDN
    #[arg(long)]
    mermaid_js: Option<PathBuf>,
}

// ---------------------------------------------------------------- site model

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Page,
    Html,
    Text,
    Other,
}

struct Entry {
    kind: Kind,
    title: String,
}

enum DirIndex {
    Entry(String),
    Generated,
}

struct Site {
    root: PathBuf,
    title: String,
    /// Source files, keyed by '/'-separated path relative to root.
    entries: BTreeMap<String, Entry>,
    /// Directories ("" is root) that get an index.
    dirs: BTreeMap<String, DirIndex>,
    /// Static output path of each entry.
    out: HashMap<String, String>,
}

fn ext_of(key: &str) -> String {
    Path::new(key)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn kind_of(key: &str) -> Kind {
    let ext = ext_of(key);
    match ext.as_str() {
        "md" | "markdown" | "mdown" => Kind::Page,
        "html" | "htm" => Kind::Html,
        e if TEXT_EXT.contains(&e) => Kind::Text,
        _ => Kind::Other,
    }
}

fn file_name(key: &str) -> &str {
    key.rsplit('/').next().unwrap_or(key)
}

fn parent_dir(key: &str) -> &str {
    key.rfind('/').map(|i| &key[..i]).unwrap_or("")
}

fn join_key(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

fn in_nav(key: &str, kind: Kind) -> bool {
    kind != Kind::Other || ext_of(key) == "pdf"
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

impl Site {
    fn nav_keys(&self) -> impl Iterator<Item = &String> {
        self.entries
            .iter()
            .filter(|(k, e)| in_nav(k, e.kind))
            .map(|(k, _)| k)
    }

    fn index_key(&self, dir: &str) -> Option<&str> {
        match self.dirs.get(dir) {
            Some(DirIndex::Entry(k)) => Some(k),
            _ => None,
        }
    }

    fn dir_out(&self, dir: &str) -> Option<String> {
        match self.dirs.get(dir)? {
            DirIndex::Entry(k) => self.out.get(k).cloned(),
            DirIndex::Generated => Some(join_key(dir, "index.html")),
        }
    }

    fn dir_id(&self, dir: &str) -> String {
        match self.dirs.get(dir) {
            Some(DirIndex::Entry(k)) => self.entry_id(k),
            _ => format!(
                "d-{}",
                if dir.is_empty() {
                    "root".into()
                } else {
                    slug(dir)
                }
            ),
        }
    }

    fn entry_id(&self, key: &str) -> String {
        match self.entries.get(key).map(|e| e.kind) {
            Some(Kind::Page) => format!("p-{}", slug(key.rsplit_once('.').map_or(key, |x| x.0))),
            _ => format!("f-{}", slug(key)),
        }
    }

    fn dir_title(&self, dir: &str) -> String {
        if dir.is_empty() {
            self.title.clone()
        } else {
            file_name(dir).to_string()
        }
    }
}

fn scan_dir(root: &Path, skip: Option<&Path>) -> Vec<String> {
    WalkDir::new(root)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| {
            if e.depth() == 0 {
                return true;
            }
            let n = e.file_name().to_string_lossy();
            if n.starts_with('.') || (e.file_type().is_dir() && SKIP_DIRS.contains(&n.as_ref())) {
                return false;
            }
            if e.file_type().is_dir() && e.path().join(OUTPUT_MARKER).exists() {
                return false;
            }
            skip.is_none_or(|s| e.path() != s)
        })
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let rel = e.path().strip_prefix(root).ok()?;
            Some(rel.to_string_lossy().replace('\\', "/"))
        })
        .collect()
}

fn read_lossy(p: &Path) -> String {
    fs::read(p)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default()
}

/// Build the site model from a directory, or by crawling links from a single file.
fn load_site(input: &Path, skip: Option<&Path>) -> Result<Site, String> {
    let input = input
        .canonicalize()
        .map_err(|e| format!("{}: {e}", input.display()))?;
    let (root, keys, start) = if input.is_dir() {
        let keys = scan_dir(&input, skip);
        (input, keys, None)
    } else {
        // Crawl from the filesystem root so links may go above the file's folder,
        // then re-root the site at the deepest directory containing everything found.
        let fs_root = input.ancestors().last().unwrap().to_path_buf();
        let start_abs = input
            .strip_prefix(&fs_root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let found = crawl(&fs_root, &start_abs);
        let mut common: Vec<&str> = parent_dir(&start_abs).split('/').collect();
        for k in &found {
            let d: Vec<&str> = parent_dir(k).split('/').collect();
            let n = common.iter().zip(&d).take_while(|(a, b)| a == b).count();
            common.truncate(n);
        }
        let prefix = common.join("/");
        let strip = |k: &str| {
            k.strip_prefix(&prefix)
                .unwrap_or(k)
                .trim_start_matches('/')
                .to_string()
        };
        let keys = found.iter().map(|k| strip(k)).collect();
        (fs_root.join(&prefix), keys, Some(strip(&start_abs)))
    };

    let mut entries = BTreeMap::new();
    for k in keys {
        let kind = kind_of(&k);
        let title = if kind == Kind::Page {
            page_title(&read_lossy(&root.join(&k))).unwrap_or_else(|| stem(&k))
        } else {
            file_name(&k).to_string()
        };
        entries.insert(k, Entry { kind, title });
    }
    if !entries.values().any(|e| e.kind == Kind::Page) {
        return Err(format!("no markdown files found in {}", root.display()));
    }

    // Every ancestor directory of a navigable entry gets an index page.
    let mut dirs = BTreeMap::new();
    dirs.insert(String::new(), DirIndex::Generated);
    for k in entries
        .iter()
        .filter(|(k, e)| in_nav(k, e.kind))
        .map(|(k, _)| k)
    {
        let mut d = parent_dir(k);
        while !d.is_empty() {
            dirs.insert(d.to_string(), DirIndex::Generated);
            d = parent_dir(d);
        }
    }
    let dir_names: Vec<String> = dirs.keys().cloned().collect();
    for d in dir_names {
        let find = |pred: &dyn Fn(&str, Kind) -> bool| {
            entries
                .iter()
                .find(|(k, e)| parent_dir(k) == d && pred(&file_name(k).to_lowercase(), e.kind))
                .map(|(k, _)| k.clone())
        };
        let idx = find(&|n, k| k == Kind::Page && n.starts_with("index."))
            .or_else(|| find(&|n, k| k == Kind::Page && n.starts_with("readme.")))
            .or_else(|| find(&|n, k| k == Kind::Html && n.starts_with("index.")));
        if let Some(i) = idx {
            dirs.insert(d, DirIndex::Entry(i));
        }
    }
    if let Some(s) = &start {
        dirs.insert(String::new(), DirIndex::Entry(s.clone()));
    }

    // Output paths. Raw files keep their path; pages become .html, avoiding clashes.
    let mut out = HashMap::new();
    let mut taken: HashSet<String> = entries
        .iter()
        .filter(|(_, e)| e.kind != Kind::Page)
        .map(|(k, _)| k.clone())
        .collect();
    for (d, idx) in &dirs {
        if matches!(idx, DirIndex::Generated) {
            taken.insert(join_key(d, "index.html"));
        }
    }
    for (k, e) in &entries {
        if e.kind != Kind::Page {
            out.insert(k.clone(), k.clone());
        }
    }
    // Page -> directory it is the index of. BTreeMap order puts "" first, so the
    // root index always lands at /index.html even when it lives in a subfolder.
    let mut index_pages: HashMap<&String, &String> = HashMap::new();
    for (d, i) in &dirs {
        if let DirIndex::Entry(k) = i {
            index_pages.entry(k).or_insert(d);
        }
    }
    for (k, e) in &entries {
        if e.kind != Kind::Page {
            continue;
        }
        let dir = parent_dir(k);
        let want = if let Some(d) = index_pages.get(k) {
            join_key(d, "index.html")
        } else {
            join_key(dir, &format!("{}.html", stem(k)))
        };
        let path = if taken.contains(&want) {
            format!("{k}.html")
        } else {
            want
        };
        taken.insert(path.clone());
        out.insert(k.clone(), path);
    }

    let title = match dirs.get("") {
        Some(DirIndex::Entry(k)) if entries[k].kind == Kind::Page => entries[k].title.clone(),
        _ => root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or("Docs".into()),
    };
    Ok(Site {
        root,
        title,
        entries,
        dirs,
        out,
    })
}

fn stem(key: &str) -> String {
    let n = file_name(key);
    n.rsplit_once('.').map_or(n, |x| x.0).to_string()
}

/// Single-file mode: include the file plus everything reachable through local links.
fn crawl(root: &Path, start: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut queue = vec![start.to_string()];
    while let Some(k) = queue.pop() {
        if !seen.insert(k.clone()) || kind_of(&k) != Kind::Page {
            continue;
        }
        let md = read_lossy(&root.join(&k));
        let arena = Arena::new();
        let doc = parse_document(&arena, &md, &md_options(None));
        visit_urls(doc, &mut |url, _| {
            if let Ok(Target::Local { key, is_dir, .. }) = resolve(root, parent_dir(&k), url) {
                if is_dir {
                    for n in ["index.md", "README.md", "readme.md"] {
                        let c = join_key(&key, n);
                        if root.join(&c).is_file() {
                            queue.push(c);
                            break;
                        }
                    }
                } else {
                    queue.push(key);
                }
            }
            None
        });
    }
    seen.into_iter().collect()
}

// ---------------------------------------------------------------- links

enum Target {
    Keep,
    SamePage(String),
    Local {
        key: String,
        is_dir: bool,
        frag: Option<String>,
    },
}

fn has_scheme(url: &str) -> bool {
    if url.starts_with("//") {
        return true;
    }
    match url.find(':') {
        Some(i) => {
            let s = &url[..i];
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
        }
        None => false,
    }
}

fn resolve(root: &Path, from_dir: &str, url: &str) -> Result<Target, String> {
    let url = url.trim();
    if url.is_empty() || has_scheme(url) {
        return Ok(Target::Keep);
    }
    if let Some(f) = url.strip_prefix('#') {
        return Ok(Target::SamePage(f.to_string()));
    }
    let (path, frag) = match url.split_once('#') {
        Some((p, f)) => (p, Some(f.to_string())),
        None => (url, None),
    };
    let path = path.split('?').next().unwrap_or("");
    let path = percent_decode(path);
    let mut parts: Vec<String> = if path.starts_with('/') {
        vec![]
    } else {
        from_dir
            .split('/')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    };
    for c in path.split('/') {
        match c {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(format!("link points outside the site: {url}"));
                }
            }
            c => parts.push(c.to_string()),
        }
    }
    let key = parts.join("/");
    let full = root.join(&key);
    if !full.exists() {
        return Err(format!("broken link: {url}"));
    }
    Ok(Target::Local {
        is_dir: full.is_dir(),
        key,
        frag,
    })
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = |c: u8| (c as char).to_digit(16);
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn percent_encode_path(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Relative href between two output files ('/'-separated, relative to site root).
fn rel_href(from_file: &str, to_file: &str) -> String {
    let from: Vec<&str> = from_file.split('/').collect();
    let from_dir = &from[..from.len() - 1];
    let to: Vec<&str> = to_file.split('/').collect();
    let mut i = 0;
    while i < from_dir.len() && i < to.len() - 1 && from_dir[i] == to[i] {
        i += 1;
    }
    let mut parts = vec![".."; from_dir.len() - i];
    parts.extend(&to[i..]);
    percent_encode_path(&parts.join("/"))
}

/// Call `f(url, is_image)` for every link/image in the document, including
/// href/src attributes in raw HTML. Returning Some replaces the URL.
fn visit_urls<'a>(doc: &'a AstNode<'a>, f: &mut dyn FnMut(&str, bool) -> Option<String>) {
    for node in doc.descendants() {
        let mut data = node.data.borrow_mut();
        match &mut data.value {
            NodeValue::Link(l) => {
                if let Some(u) = f(&l.url, false) {
                    l.url = u;
                }
            }
            NodeValue::Image(l) => {
                if let Some(u) = f(&l.url, true) {
                    l.url = u;
                }
            }
            NodeValue::HtmlBlock(h) => h.literal = rewrite_html_attrs(&h.literal, f),
            NodeValue::HtmlInline(s) => *s = rewrite_html_attrs(s, f),
            _ => {}
        }
    }
}

fn rewrite_html_attrs(html: &str, f: &mut dyn FnMut(&str, bool) -> Option<String>) -> String {
    let lower = html.to_ascii_lowercase();
    let b = html.as_bytes();
    let mut out = String::new();
    let mut last = 0;
    let mut i = 0;
    while i < b.len() {
        let attr = if lower[i..].starts_with("href") {
            4
        } else if lower[i..].starts_with("src") {
            3
        } else {
            i += 1;
            continue;
        };
        if i == 0 || !b[i - 1].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let is_img = attr == 3;
        let mut j = i + attr;
        while j < b.len() && b[j].is_ascii_whitespace() {
            j += 1;
        }
        if j >= b.len() || b[j] != b'=' {
            i += 1;
            continue;
        }
        j += 1;
        while j < b.len() && b[j].is_ascii_whitespace() {
            j += 1;
        }
        if j >= b.len() || (b[j] != b'"' && b[j] != b'\'') {
            i += 1;
            continue;
        }
        let q = b[j];
        let start = j + 1;
        let Some(len) = b[start..].iter().position(|&c| c == q) else {
            break;
        };
        let end = start + len;
        if let Some(new) = f(&html[start..end].replace("&amp;", "&"), is_img) {
            out.push_str(&html[last..start]);
            out.push_str(&esc(&new));
            last = end;
        }
        i = end + 1;
    }
    out.push_str(&html[last..]);
    out
}

// ---------------------------------------------------------------- rendering

fn md_options(id_prefix: Option<String>) -> Options<'static> {
    let mut o = Options::default();
    o.extension.strikethrough = true;
    o.extension.table = true;
    o.extension.autolink = true;
    o.extension.tasklist = true;
    o.extension.footnotes = true;
    o.extension.alerts = true;
    o.extension.header_id_prefix = id_prefix;
    o.extension.header_id_prefix_in_href = true;
    o.render.r#unsafe = true;
    o.render.github_pre_lang = true;
    o
}

fn page_title(md: &str) -> Option<String> {
    let arena = Arena::new();
    let doc = parse_document(&arena, md, &md_options(None));
    let h1 = doc
        .descendants()
        .find(|n| matches!(n.data.borrow().value, NodeValue::Heading(ref h) if h.level == 1))?;
    let mut t = String::new();
    for n in h1.descendants() {
        match &n.data.borrow().value {
            NodeValue::Text(s) => t.push_str(s),
            NodeValue::Code(c) => t.push_str(&c.literal),
            _ => {}
        }
    }
    let t = t.trim().to_string();
    (!t.is_empty()).then_some(t)
}

// ---------------------------------------------------------------- syntax highlighting

const HL_PREFIX: &str = "hl-";
const HL_LIGHT: &str = "InspiredGitHub";
const HL_DARK: &str = "base16-ocean.dark";

fn highlighter() -> &'static SyntectAdapter {
    static H: OnceLock<SyntectAdapter> = OnceLock::new();
    H.get_or_init(|| {
        SyntectAdapterBuilder::new()
            .css_with_class_prefix(HL_PREFIX)
            .build()
    })
}

/// Class-based highlight CSS: a light theme, and a dark one under prefers-color-scheme.
fn highlight_css() -> &'static str {
    static CSS: OnceLock<String> = OnceLock::new();
    CSS.get_or_init(|| {
        use syntect::highlighting::ThemeSet;
        use syntect::html::{css_for_theme_with_class_style, ClassStyle};
        let themes = ThemeSet::load_defaults();
        let style = ClassStyle::SpacedPrefixed { prefix: HL_PREFIX };
        let css = |name: &str| {
            let raw = css_for_theme_with_class_style(&themes.themes[name], style).unwrap_or_default();
            // Keep our own code-block background; only take the token colors.
            raw.lines()
                .filter(|l| !l.trim_start().starts_with("background-color"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        // Each theme in its own media query so neither leaks rules into the other.
        format!(
            "@media (prefers-color-scheme: light) {{\n{}\n}}\n@media (prefers-color-scheme: dark) {{\n{}\n}}\n",
            css(HL_LIGHT),
            css(HL_DARK)
        )
    })
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn mime_of(key: &str) -> &'static str {
    match ext_of(key).as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wasm" => "application/wasm",
        e if TEXT_EXT.contains(&e) || e == "md" || e == "markdown" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Static,
    Single,
}

/// Rendering context for one output page.
struct Ctx<'a> {
    site: &'a Site,
    mode: Mode,
    /// Static output path of the page being rendered.
    from_out: String,
    /// Single-mode section id of the page being rendered.
    from_id: String,
    /// Single mode: files that need to be embedded as downloadable blobs.
    assets: &'a RefCell<BTreeSet<String>>,
    warnings: &'a RefCell<Vec<String>>,
}

impl Ctx<'_> {
    fn href(&self, key: &str, is_dir: bool, frag: Option<&str>) -> Option<String> {
        let site = self.site;
        match self.mode {
            Mode::Static => {
                let out = if is_dir {
                    site.dir_out(key)?
                } else {
                    site.out.get(key)?.clone()
                };
                let mut h = rel_href(&self.from_out, &out);
                if let Some(f) = frag {
                    h = format!("{h}#{f}");
                }
                Some(h)
            }
            Mode::Single => {
                if is_dir {
                    site.dirs.get(key)?;
                    return Some(format!("#{}", site.dir_id(key)));
                }
                let e = site.entries.get(key)?;
                Some(match (e.kind, frag) {
                    (Kind::Page, Some(f)) if !f.is_empty() => {
                        format!("#{}--{f}", site.entry_id(key))
                    }
                    (Kind::Page | Kind::Text | Kind::Html, _) => format!("#{}", site.entry_id(key)),
                    (Kind::Other, _) => {
                        self.assets.borrow_mut().insert(key.to_string());
                        format!("#asset:{}", percent_encode_path(key))
                    }
                })
            }
        }
    }

    fn rewrite(&self, from_dir: &str, url: &str, is_img: bool) -> Option<String> {
        let site = self.site;
        match resolve(&site.root, from_dir, url) {
            Ok(Target::Keep) => None,
            Ok(Target::SamePage(f)) => match self.mode {
                Mode::Single if !f.is_empty() => Some(format!("#{}--{f}", self.from_id)),
                _ => None,
            },
            Ok(Target::Local { key, is_dir, frag }) => {
                if self.mode == Mode::Single && is_img && !is_dir {
                    let bytes = fs::read(site.root.join(&key)).ok()?;
                    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
                    return Some(format!("data:{};base64,{b64}", mime_of(&key)));
                }
                if !is_dir && !site.entries.contains_key(&key) {
                    self.warn(format!("{url}: target is not part of the site"));
                    return None;
                }
                let h = self.href(&key, is_dir, frag.as_deref());
                if h.is_none() {
                    self.warn(format!("{url}: no page for this directory"));
                }
                h
            }
            Err(w) => {
                self.warn(w);
                None
            }
        }
    }

    fn warn(&self, w: String) {
        self.warnings.borrow_mut().push(w);
    }
}

fn render_markdown(ctx: &Ctx, key: &str) -> String {
    let md = read_lossy(&ctx.site.root.join(key));
    let prefix = match ctx.mode {
        Mode::Static => String::new(),
        Mode::Single => format!("{}--", ctx.from_id),
    };
    let opts = md_options(Some(prefix.clone()));
    let arena = Arena::new();
    let doc = parse_document(&arena, &md, &opts);
    let dir = parent_dir(key);
    visit_urls(doc, &mut |url, is_img| ctx.rewrite(dir, url, is_img));
    for node in doc.descendants() {
        let mut data = node.data.borrow_mut();
        let replacement = match &data.value {
            NodeValue::CodeBlock(cb) if cb.info.split_whitespace().next() == Some("mermaid") => {
                Some(NodeValue::HtmlBlock(NodeHtmlBlock {
                    block_type: 6,
                    literal: format!("<pre class=\"mermaid\">{}</pre>\n", esc(&cb.literal)),
                }))
            }
            _ => None,
        };
        if let Some(r) = replacement {
            data.value = r;
        }
    }
    let mut plugins = Plugins::default();
    plugins.render.codefence_syntax_highlighter = Some(highlighter());
    let mut html = String::new();
    format_html_with_plugins(doc, &opts, &mut html, &plugins).expect("html formatting");
    if ctx.mode == Mode::Single {
        // Footnote ids would collide between pages.
        html = html
            .replace("id=\"fn", &format!("id=\"{prefix}fn"))
            .replace("href=\"#fn", &format!("href=\"#{prefix}fn"));
    }
    html
}

fn render_listing(ctx: &Ctx, dir: &str) -> String {
    let site = ctx.site;
    let mut h = format!(
        "<h1>{}</h1>\n<ul class=\"listing\">\n",
        esc(&site.dir_title(dir))
    );
    for sub in site
        .dirs
        .keys()
        .filter(|d| !d.is_empty() && parent_dir(d) == dir)
    {
        if let Some(href) = ctx.href(sub, true, None) {
            h += &format!(
                "<li><a href=\"{}\">{}/</a></li>\n",
                esc(&href),
                esc(file_name(sub))
            );
        }
    }
    for k in site.nav_keys().filter(|k| parent_dir(k) == dir) {
        if let Some(href) = ctx.href(k, false, None) {
            h += &format!(
                "<li><a href=\"{}\">{}</a>{}</li>\n",
                esc(&href),
                esc(&site.entries[k].title),
                tag(k, site.entries[k].kind)
            );
        }
    }
    h + "</ul>\n"
}

fn tag(key: &str, kind: Kind) -> String {
    if kind == Kind::Page {
        String::new()
    } else {
        format!(" <span class=\"tag\">{}</span>", esc(&ext_of(key)))
    }
}

/// Sidebar navigation tree. `active` is the static output path of the current page
/// (single mode passes None and highlights client-side).
fn render_nav(ctx: &Ctx, active: Option<&str>) -> String {
    let site = ctx.site;
    let link = |key: &str, is_dir: bool, label: &str, extra: &str| {
        let out = if is_dir {
            site.dir_out(key)
        } else {
            site.out.get(key).cloned()
        };
        let act = active.is_some() && out.as_deref() == active;
        format!(
            "<a href=\"{}\"{}>{}{}</a>",
            esc(&ctx.href(key, is_dir, None).unwrap_or_default()),
            if act { " class=\"active\"" } else { "" },
            esc(label),
            extra
        )
    };
    let root = link("", true, &site.title, "").replacen("<a ", "<a data-site ", 1);
    format!("{root}\n{}", nav_dir(site, "", &link))
}

fn nav_dir(site: &Site, dir: &str, link: &dyn Fn(&str, bool, &str, &str) -> String) -> String {
    let idx = site.index_key(dir);
    let mut h = String::from("<ul>\n");
    for k in site
        .nav_keys()
        .filter(|k| parent_dir(k) == dir && Some(k.as_str()) != idx)
    {
        let e = &site.entries[k];
        h += &format!("<li>{}</li>\n", link(k, false, &e.title, &tag(k, e.kind)));
    }
    for sub in site
        .dirs
        .keys()
        .filter(|d| !d.is_empty() && parent_dir(d) == dir)
    {
        h += &format!(
            "<li><details open><summary>{}</summary>\n{}</details></li>\n",
            link(sub, true, file_name(sub), ""),
            nav_dir(site, sub, link)
        );
    }
    h + "</ul>\n"
}

fn show_nav(site: &Site) -> bool {
    site.nav_keys().count() > 1
}

struct Page<'a> {
    title: &'a str,
    nav: Option<String>,
    body: &'a str,
    head: &'a str,
    tail: &'a str,
}

fn page_html(p: &Page) -> String {
    let nav = match &p.nav {
        Some(n) => format!("<nav class=\"side\">\n{n}</nav>\n"),
        None => String::new(),
    };
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{}</title>\n<style>{STYLE}{}</style>\n{}</head>\n<body>\n{nav}<main>\n{}</main>\n{}</body>\n</html>\n",
        esc(p.title),
        if p.body.contains("class=\"hl-") {
            highlight_css()
        } else {
            ""
        },
        p.head,
        p.body,
        p.tail
    )
}

fn mermaid_script(src: &str, single: bool) -> String {
    let init = if single {
        "window.mdsiteMermaid = (el) => mermaid.run({nodes: el.querySelectorAll('pre.mermaid:not([data-processed])')});"
    } else {
        "mermaid.run();"
    };
    format!(
        "{src}\n<script>\nmermaid.initialize({{startOnLoad: false, theme: matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'default'}});\n{init}\n</script>\n"
    )
}

// ---------------------------------------------------------------- outputs

struct BuildOpts {
    mermaid_js: Option<PathBuf>,
    live_reload: bool,
}

const LIVE_RELOAD: &str = "<script>(()=>{let v=null;setInterval(async()=>{try{const t=await (await fetch('/__mdsite/version')).text();if(v!==null&&t!==v)location.reload();v=t}catch(e){}},700)})();</script>\n";

/// Writes the site into `out_dir`; returns the number of files written.
fn build_static(site: &Site, out_dir: &Path, opts: &BuildOpts) -> Result<usize, String> {
    let assets = RefCell::new(BTreeSet::new());
    let warnings = RefCell::new(Vec::new());
    let io = |e: std::io::Error| e.to_string();
    let write = |rel: &str, bytes: &[u8]| -> Result<(), String> {
        let p = out_dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).map_err(io)?;
        fs::write(&p, bytes).map_err(|e| format!("{}: {e}", p.display()))
    };

    write(OUTPUT_MARKER, b"generated by mdsite\n")?;
    if let Some(js) = &opts.mermaid_js {
        write(
            "_mdsite/mermaid.min.js",
            &fs::read(js).map_err(|e| format!("{}: {e}", js.display()))?,
        )?;
    }
    let tail_for = |from_out: &str, body: &str| {
        let mut t = String::new();
        if body.contains("class=\"mermaid\"") {
            let src = match &opts.mermaid_js {
                Some(_) => format!(
                    "<script src=\"{}\"></script>",
                    rel_href(from_out, "_mdsite/mermaid.min.js")
                ),
                None => format!("<script src=\"{MERMAID_CDN}\"></script>"),
            };
            t += &mermaid_script(&src, false);
        }
        if opts.live_reload {
            t += LIVE_RELOAD;
        }
        t
    };

    let render = |from_out: &str,
                  from_label: &str,
                  title: &str,
                  body_fn: &dyn Fn(&Ctx) -> String|
     -> Result<(), String> {
        let ctx = Ctx {
            site,
            mode: Mode::Static,
            from_out: from_out.into(),
            from_id: String::new(),
            assets: &assets,
            warnings: &warnings,
        };
        let body = format!("<article>\n{}</article>\n", body_fn(&ctx));
        let nav = show_nav(site).then(|| render_nav(&ctx, Some(from_out)));
        let tail = tail_for(from_out, &body);
        let full_title = if title == site.title {
            title.to_string()
        } else {
            format!("{title} · {}", site.title)
        };
        write(
            from_out,
            page_html(&Page {
                title: &full_title,
                nav,
                body: &body,
                head: "",
                tail: &tail,
            })
            .as_bytes(),
        )?;
        for w in warnings.borrow_mut().drain(..) {
            eprintln!("warning: {from_label}: {w}");
        }
        Ok(())
    };

    for (k, e) in &site.entries {
        let out = &site.out[k];
        if e.kind == Kind::Page {
            render(out, k, &e.title, &|ctx| render_markdown(ctx, k))?;
        } else {
            write(out, &fs::read(site.root.join(k)).map_err(io)?)?;
        }
    }
    for (d, idx) in &site.dirs {
        if matches!(idx, DirIndex::Generated) {
            let out = join_key(d, "index.html");
            render(&out, &out, &site.dir_title(d), &|ctx| {
                render_listing(ctx, d)
            })?;
        }
    }
    let generated = site
        .dirs
        .values()
        .filter(|i| matches!(i, DirIndex::Generated))
        .count();
    Ok(site.entries.len() + generated)
}

fn build_single(site: &Site, opts: &BuildOpts) -> Result<String, String> {
    let assets = RefCell::new(BTreeSet::new());
    let warnings = RefCell::new(Vec::new());
    let ctx_for = |id: String| Ctx {
        site,
        mode: Mode::Single,
        from_out: String::new(),
        from_id: id,
        assets: &assets,
        warnings: &warnings,
    };
    let section = |id: &str, title: &str, inner: &str| {
        format!(
            "<section class=\"page\" id=\"{}\" data-title=\"{}\" hidden>\n<article>\n{inner}</article>\n</section>\n",
            esc(id),
            esc(title)
        )
    };
    let flush = |label: &str| {
        for w in warnings.borrow_mut().drain(..) {
            eprintln!("warning: {label}: {w}");
        }
    };

    let mut body = String::new();
    for (d, idx) in &site.dirs {
        if matches!(idx, DirIndex::Generated) {
            let id = site.dir_id(d);
            body += &section(
                &id,
                &site.dir_title(d),
                &render_listing(&ctx_for(id.clone()), d),
            );
        }
    }
    for (k, e) in &site.entries {
        let id = site.entry_id(k);
        let inner = match e.kind {
            Kind::Page => render_markdown(&ctx_for(id.clone()), k),
            Kind::Text => format!(
                "<h1>{}</h1>\n<pre class=\"file\">{}</pre>\n",
                esc(file_name(k)),
                esc(&read_lossy(&site.root.join(k)))
            ),
            Kind::Html => {
                assets.borrow_mut().insert(k.clone());
                format!(
                    "<h1>{} <a class=\"open\" href=\"#asset:{}\">open ↗</a></h1>\n<iframe class=\"file\" srcdoc=\"{}\"></iframe>\n",
                    esc(file_name(k)),
                    percent_encode_path(k),
                    esc(&read_lossy(&site.root.join(k)))
                )
            }
            Kind::Other => continue,
        };
        body += &section(&id, &e.title, &inner);
        flush(k);
    }
    // PDFs etc. listed in nav are opened as blobs.
    for k in site.nav_keys() {
        if site.entries[k].kind == Kind::Other {
            assets.borrow_mut().insert(k.clone());
        }
    }

    let ctx = ctx_for(String::new());
    let nav = show_nav(site).then(|| render_nav(&ctx, None));
    flush("nav");

    let mut asset_json = String::from("{");
    for (i, k) in assets.borrow().iter().enumerate() {
        let bytes = fs::read(site.root.join(k)).map_err(|e| format!("{k}: {e}"))?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        let sep = if i > 0 { "," } else { "" };
        asset_json += &format!("{sep}{}:[{},\"{b64}\"]", json_str(k), json_str(mime_of(k)));
    }
    asset_json += "}";

    let mut tail = format!(
        "<script type=\"application/json\" id=\"mdsite-assets\">{}</script>\n<script>const MDSITE_FIRST = {};\n{SINGLE_JS}</script>\n",
        asset_json.replace("</", "<\\/"),
        json_str(&site.dir_id(""))
    );
    if body.contains("class=\"mermaid\"") {
        let src = match &opts.mermaid_js {
            Some(p) => {
                let js = read_lossy(p);
                format!("<script>{}</script>", js.replace("</script", "<\\/script"))
            }
            None => format!("<script src=\"{MERMAID_CDN}\"></script>"),
        };
        tail = mermaid_script(&src, true) + &tail;
    }
    Ok(page_html(&Page {
        title: &site.title,
        nav,
        body: &body,
        head: "",
        tail: &tail,
    }))
}

fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o += "\\\"",
            '\\' => o += "\\\\",
            '<' => o += "\\u003c",
            c if (c as u32) < 0x20 => o += &format!("\\u{:04x}", c as u32),
            c => o.push(c),
        }
    }
    o + "\""
}

// ---------------------------------------------------------------- serve

fn fingerprint(input: &Path, skip: Option<&Path>) -> (usize, SystemTime) {
    let mtime = |p: &Path| {
        fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
    };
    if input.is_file() {
        let dir = input.parent().unwrap();
        let files = scan_dir(dir, skip);
        let newest = files
            .iter()
            .map(|k| mtime(&dir.join(k)))
            .max()
            .unwrap_or(SystemTime::UNIX_EPOCH);
        return (files.len(), newest);
    }
    let files = scan_dir(input, skip);
    let newest = files
        .iter()
        .map(|k| mtime(&input.join(k)))
        .max()
        .unwrap_or(SystemTime::UNIX_EPOCH);
    (files.len(), newest)
}

fn serve(input: PathBuf, port: u16, opts: BuildOpts) -> Result<(), String> {
    let tmp = std::env::temp_dir().join(format!("mdsite-{}", std::process::id()));
    let watch_input = input.clone();
    let rebuild = {
        let tmp = tmp.clone();
        move || -> Result<(), String> {
            let _ = fs::remove_dir_all(&tmp);
            let site = load_site(&input, None)?;
            build_static(&site, &tmp, &opts)?;
            Ok(())
        }
    };
    rebuild()?;
    let version = Arc::new(AtomicU64::new(1));
    {
        let version = version.clone();
        let input = watch_input;
        std::thread::spawn(move || {
            let mut last = fingerprint(&input, None);
            loop {
                std::thread::sleep(Duration::from_millis(400));
                let now = fingerprint(&input, None);
                if now != last {
                    last = now;
                    match rebuild() {
                        Ok(()) => eprintln!("rebuilt"),
                        Err(e) => eprintln!("error: {e}"),
                    }
                    version.fetch_add(1, Ordering::SeqCst);
                }
            }
        });
    }

    let server = tiny_http::Server::http(("127.0.0.1", port)).map_err(|e| e.to_string())?;
    eprintln!("serving on http://127.0.0.1:{port}/");
    for req in server.incoming_requests() {
        let url = req
            .url()
            .split(['?', '#'])
            .next()
            .unwrap_or("/")
            .to_string();
        if url == "/__mdsite/version" {
            let _ = req.respond(tiny_http::Response::from_string(
                version.load(Ordering::SeqCst).to_string(),
            ));
            continue;
        }
        let rel = percent_decode(url.trim_start_matches('/'));
        let resp = if rel.split('/').any(|c| c == "..") {
            None
        } else {
            let mut p = tmp.join(&rel);
            if p.is_dir() {
                p = p.join("index.html");
            }
            fs::read(&p)
                .ok()
                .map(|b| (b, mime_of(&p.to_string_lossy())))
        };
        let _ = match resp {
            Some((bytes, mime)) => req.respond(
                tiny_http::Response::from_data(bytes)
                    .with_header(tiny_http::Header::from_bytes("Content-Type", mime).unwrap()),
            ),
            None => {
                req.respond(tiny_http::Response::from_string("not found").with_status_code(404))
            }
        };
    }
    Ok(())
}

// ---------------------------------------------------------------- main

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    let opts = BuildOpts {
        mermaid_js: cli.mermaid_js.clone(),
        live_reload: cli.serve,
    };
    if cli.serve {
        return serve(cli.input, cli.port, opts);
    }
    if cli.single {
        let site = load_site(&cli.input, None)?;
        let out = cli.out.unwrap_or_else(|| {
            let name = cli
                .input
                .canonicalize()
                .ok()
                .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()));
            PathBuf::from(format!("{}.html", name.unwrap_or("site".into())))
        });
        fs::write(&out, build_single(&site, &opts)?)
            .map_err(|e| format!("{}: {e}", out.display()))?;
        eprintln!("wrote {}", out.display());
    } else {
        let out = cli.out.unwrap_or_else(|| PathBuf::from("site"));
        fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
        let out = out.canonicalize().map_err(|e| e.to_string())?;
        let site = load_site(&cli.input, Some(&out))?;
        let n = build_static(&site, &out, &opts)?;
        eprintln!(
            "wrote {n} file{} to {}",
            if n == 1 { "" } else { "s" },
            out.display()
        );
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
