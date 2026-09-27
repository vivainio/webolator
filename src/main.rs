//! webolator: turn a markdown file or a folder of markdown into a browsable site.
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

const MERMAID_CDN: &str = "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js";
const STYLE: &str = include_str!("style.css");
const SINGLE_JS: &str = include_str!("single.js");
const COMMON_JS: &str = include_str!("common.js");

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
const OUTPUT_MARKER: &str = ".webolator-output";
/// A stylesheet with this name in the site root is applied automatically.
const CUSTOM_CSS: &str = "webolator.css";
const SKIP_DIRS: &[&str] = &["node_modules", "target", "__pycache__", "venv"];

#[derive(Parser)]
#[command(
    version,
    about = "Render markdown files into a zero-config website (with mermaid)"
)]
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
    /// Extra stylesheet applied after the built-in styles (in addition to <root>/webolator.css)
    #[arg(long)]
    css: Option<PathBuf>,
    /// Leave out files matching this gitignore-style pattern (repeatable)
    #[arg(long, value_name = "GLOB")]
    exclude: Vec<String>,
    /// Include files that .gitignore / .ignore would leave out (.webolatorignore still applies)
    #[arg(long)]
    no_ignore: bool,
    /// Don't add "last updated" dates and "edit this page" links from git
    #[arg(long)]
    no_git: bool,
    /// Fail (exit code 1) if there are broken or excluded links
    #[arg(long)]
    check: bool,
    /// Show this folder as a file list instead of rendering its contents (repeatable)
    #[arg(long, value_name = "DIR")]
    files: Vec<PathBuf>,
}

static WARNINGS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn print_warning(label: &str, w: &str) {
    WARNINGS.fetch_add(1, Ordering::SeqCst);
    eprintln!("warning: {label}: {w}");
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
    /// Sort position from front matter `order:`; overrides a "01-" filename prefix.
    order: Option<i64>,
    /// Front matter `hidden: true`: built and linkable, but not in the sidebar or prev/next.
    hidden: bool,
}

enum DirIndex {
    Entry(String),
    Generated,
    /// A --files folder (or a folder inside one): a file list at this output path.
    Files(String),
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
    git: Option<GitInfo>,
}

/// A page in reading order (sidebar order, depth first).
#[derive(Clone, PartialEq)]
enum Doc {
    Entry(String),
    Listing(String),
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
            .filter(|(k, e)| in_nav(k, e.kind) && !e.hidden)
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
            DirIndex::Generated => Some(join_key(&clean_dir(dir), "index.html")),
            DirIndex::Files(out) => Some(out.clone()),
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

    /// Navigable entries and subdirectories directly in `dir`, as (key, is_dir).
    /// Ordered by numeric prefix ("01-"), then files before folders, then name.
    fn children(&self, dir: &str) -> Vec<(String, bool)> {
        let mut v: Vec<(String, bool)> = self
            .nav_keys()
            .filter(|k| parent_dir(k) == dir)
            .map(|k| (k.clone(), false))
            .chain(
                self.dirs
                    .keys()
                    .filter(|d| !d.is_empty() && parent_dir(d) == dir)
                    .map(|d| (d.clone(), true)),
            )
            .collect();
        v.sort_by_cached_key(|(k, is_dir)| {
            let (n, rest) = order_prefix(file_name(k));
            // A folder takes the front matter order of its index page.
            let meta_key = if *is_dir {
                self.index_key(k)
            } else {
                Some(k.as_str())
            };
            let order = meta_key
                .and_then(|m| self.entries.get(m))
                .and_then(|e| e.order)
                .or(n.map(|n| n as i64));
            (order.unwrap_or(i64::MAX), *is_dir, rest.to_lowercase())
        });
        v
    }

    /// Every page we render, in sidebar order: each folder's index, then its contents.
    fn reading_order(&self) -> Vec<Doc> {
        fn walk(site: &Site, dir: &str, out: &mut Vec<Doc>) {
            match site.dirs.get(dir) {
                Some(DirIndex::Entry(k)) if site.entries[k].kind == Kind::Page => {
                    out.push(Doc::Entry(k.clone()))
                }
                Some(DirIndex::Generated) => out.push(Doc::Listing(dir.to_string())),
                // A file list is one stop in the reading order; its contents aren't pages.
                Some(DirIndex::Files(_)) => return out.push(Doc::Listing(dir.to_string())),
                _ => {}
            }
            let idx = site.index_key(dir);
            for (k, is_dir) in site.children(dir) {
                if is_dir {
                    walk(site, &k, out);
                } else if Some(k.as_str()) != idx && site.entries[&k].kind == Kind::Page {
                    out.push(Doc::Entry(k));
                }
            }
        }
        let mut v = Vec::new();
        walk(self, "", &mut v);
        v
    }

    /// Resolve a [[wikilink]] name: a path ("guide/setup"), a file name ("setup",
    /// "01-setup", "Setup Guide"), or a page title. Case, spaces and '_' vs '-' don't matter.
    fn find_wiki(&self, name: &str) -> Option<&str> {
        let norm = |s: &str| {
            let s = s.trim().to_lowercase().replace([' ', '_'], "-");
            s.strip_suffix(".md").map(str::to_string).unwrap_or(s)
        };
        let want = norm(name);
        let pages = || self.entries.iter().filter(|(_, e)| e.kind == Kind::Page);
        let no_ext = |k: &str| k.rsplit_once('.').map_or(k, |x| x.0).to_string();
        pages()
            .find(|(k, _)| norm(&no_ext(k)) == want || norm(&clean_dir(&no_ext(k))) == want)
            .or_else(|| {
                pages().find(|(k, _)| {
                    let st = stem(k);
                    norm(&st) == want || norm(display_name(&st)) == want
                })
            })
            .or_else(|| pages().find(|(_, e)| norm(&e.title) == want))
            .map(|(k, _)| k.as_str())
    }

    fn dir_title(&self, dir: &str) -> String {
        if dir.is_empty() {
            self.title.clone()
        } else {
            display_name(file_name(dir)).to_string()
        }
    }
}

/// Which files a folder scan leaves out, beyond the built-in rules.
#[derive(Clone, Default)]
struct ScanOpts {
    /// A directory to leave out entirely (the output directory).
    skip: Option<PathBuf>,
    /// Don't honor .gitignore / .ignore files.
    no_ignore: bool,
    /// Extra gitignore-style patterns from --exclude.
    exclude: Vec<String>,
    /// Look up "last updated" dates and edit links in git.
    git: bool,
    /// Folders (from --files) whose contents are listed, not rendered.
    files: Vec<PathBuf>,
}

/// Name of the gitignore-syntax file listing what to leave out of the site.
const IGNORE_FILE: &str = ".webolatorignore";

/// All publishable files under `root`, as sorted '/'-separated relative paths.
/// Skips hidden files, SKIP_DIRS, earlier output dirs, and anything matched by
/// .gitignore / .ignore (unless no_ignore), .webolatorignore or --exclude.
fn scan_dir(root: &Path, opts: &ScanOpts) -> Result<Vec<String>, String> {
    let mut overrides = ignore::overrides::OverrideBuilder::new(root);
    for pat in &opts.exclude {
        overrides
            .add(&format!("!{pat}"))
            .map_err(|e| format!("--exclude {pat}: {e}"))?;
    }
    let overrides = overrides.build().map_err(|e| e.to_string())?;
    let honor_git = !opts.no_ignore;
    let skip = opts.skip.clone();
    let walker = ignore::WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(honor_git)
        .git_exclude(honor_git)
        .git_global(honor_git)
        .ignore(honor_git)
        .parents(honor_git)
        .require_git(false)
        .add_custom_ignore_filename(IGNORE_FILE)
        .overrides(overrides)
        .sort_by_file_name(|a, b| a.cmp(b))
        .filter_entry(move |e| {
            if e.depth() == 0 {
                return true;
            }
            let is_dir = e.file_type().is_some_and(|t| t.is_dir());
            let n = e.file_name().to_string_lossy();
            if is_dir && SKIP_DIRS.contains(&n.as_ref()) {
                return false;
            }
            if is_dir && e.path().join(OUTPUT_MARKER).exists() {
                return false;
            }
            skip.as_deref().is_none_or(|s| e.path() != s)
        })
        .build();
    Ok(walker
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
        .filter_map(|e| {
            let rel = e.path().strip_prefix(root).ok()?;
            Some(rel.to_string_lossy().replace('\\', "/"))
        })
        .collect())
}

fn read_lossy(p: &Path) -> String {
    fs::read(p)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default()
}

/// Build the site model from a directory, or by crawling links from a single file.
fn load_site(input: &Path, scan: &ScanOpts) -> Result<Site, String> {
    let input = input
        .canonicalize()
        .map_err(|e| format!("{}: {e}", input.display()))?;
    // --files folders: relative to the input folder (or the input file's folder),
    // falling back to the current directory.
    let base = if input.is_dir() {
        input.clone()
    } else {
        input.parent().unwrap().to_path_buf()
    };
    let files_abs = scan
        .files
        .iter()
        .map(|p| {
            let cand = if base.join(p).is_dir() {
                base.join(p)
            } else {
                p.clone()
            };
            cand.canonicalize()
                .ok()
                .filter(|c| c.is_dir())
                .ok_or_else(|| format!("--files {}: not a folder", p.display()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (root, keys, start) = if input.is_dir() {
        let keys = scan_dir(&input, scan)?;
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
        let mut found = crawl(&fs_root, &start_abs);
        // --files folders are included whole, whether or not anything links to them.
        let mut files_rel = Vec::new();
        for fd in &files_abs {
            let rel = fd
                .strip_prefix(&fs_root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            found.extend(scan_dir(fd, scan)?.into_iter().map(|k| join_key(&rel, &k)));
            files_rel.push(join_key(&rel, "x"));
        }
        let mut common: Vec<&str> = parent_dir(&start_abs).split('/').collect();
        for k in found.iter().chain(&files_rel) {
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

    let files_dirs = files_abs
        .iter()
        .map(|fd| {
            fd.strip_prefix(&root)
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .map_err(|_| format!("--files {}: outside the site folder", fd.display()))
        })
        .collect::<Result<Vec<String>, _>>()?;
    let in_files_dir = |k: &str| {
        files_dirs
            .iter()
            .any(|fd| fd.is_empty() || k == fd || k.starts_with(&format!("{fd}/")))
    };

    let mut entries = BTreeMap::new();
    for k in keys.into_iter().filter(|k| k != CUSTOM_CSS) {
        let kind = kind_of(&k);
        let entry = if in_files_dir(&k) {
            // Copied as is and only shown in the folder's file list.
            Entry {
                kind: Kind::Other,
                title: file_name(&k).to_string(),
                order: None,
                hidden: true,
            }
        } else if kind == Kind::Page {
            let m = page_meta(&read_lossy(&root.join(&k)));
            Entry {
                kind,
                title: m
                    .title
                    .unwrap_or_else(|| display_name(&stem(&k)).to_string()),
                order: m.order,
                hidden: m.hidden,
            }
        } else {
            Entry {
                kind,
                title: file_name(&k).to_string(),
                order: None,
                hidden: false,
            }
        };
        entries.insert(k, entry);
    }
    if files_dirs.is_empty() && !entries.values().any(|e| e.kind == Kind::Page) {
        return Err(format!("no markdown files found in {}", root.display()));
    }

    // Every ancestor directory of a navigable entry gets an index page.
    let mut dirs = BTreeMap::new();
    dirs.insert(String::new(), DirIndex::Generated);
    for k in entries
        .iter()
        .filter(|(k, e)| in_nav(k, e.kind) && !e.hidden)
        .map(|(k, _)| k)
    {
        let mut d = parent_dir(k);
        while !d.is_empty() {
            dirs.insert(d.to_string(), DirIndex::Generated);
            d = parent_dir(d);
        }
    }
    // --files folders and every folder inside them get a file list; the folders
    // above them get normal index pages.
    let mut files_listing_dirs = BTreeSet::new();
    for fd in &files_dirs {
        files_listing_dirs.insert(fd.clone());
        for k in entries.keys().filter(|k| in_files_dir(k)) {
            let mut d = parent_dir(k);
            while d.len() > fd.len() {
                files_listing_dirs.insert(d.to_string());
                d = parent_dir(d);
            }
        }
        let mut d = parent_dir(fd);
        while !fd.is_empty() {
            dirs.entry(d.to_string()).or_insert(DirIndex::Generated);
            if d.is_empty() {
                break;
            }
            d = parent_dir(d);
        }
    }
    for d in &files_listing_dirs {
        dirs.insert(d.clone(), DirIndex::Files(String::new()));
    }
    let dir_names: Vec<String> = dirs
        .iter()
        .filter(|(_, i)| !matches!(i, DirIndex::Files(_)))
        .map(|(d, _)| d.clone())
        .collect();
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
        .map(|(k, _)| join_key(&raw_aware_dir(&files_dirs, parent_dir(k)), file_name(k)))
        .collect();
    for (d, idx) in dirs.iter_mut() {
        match idx {
            DirIndex::Generated => {
                taken.insert(join_key(&clean_dir(d), "index.html"));
            }
            DirIndex::Files(out) => {
                // Don't overwrite an index.html that is itself one of the listed files.
                let dir_out = raw_aware_dir(&files_dirs, d);
                let want = join_key(&dir_out, "index.html");
                *out = if taken.contains(&want) {
                    join_key(&dir_out, "_files.html")
                } else {
                    want
                };
                taken.insert(out.clone());
            }
            DirIndex::Entry(_) => {}
        }
    }
    for (k, e) in &entries {
        if e.kind != Kind::Page {
            out.insert(
                k.clone(),
                join_key(&raw_aware_dir(&files_dirs, parent_dir(k)), file_name(k)),
            );
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
            join_key(&clean_dir(d), "index.html")
        } else {
            join_key(&clean_dir(dir), &format!("{}.html", display_name(&stem(k))))
        };
        let path = if taken.contains(&want) {
            join_key(&clean_dir(dir), &format!("{}.html", file_name(k)))
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
    let git = if scan.git { git_info(&root) } else { None };
    Ok(Site {
        root,
        title,
        entries,
        dirs,
        out,
        git,
    })
}

/// Split an ordering prefix off a file or folder name: "01-usage.md" -> (Some(1), "usage.md").
/// The prefix is digits followed by '-', '_' or ' ', and only counts if a name follows.
fn order_prefix(name: &str) -> (Option<u64>, &str) {
    let digits = name.bytes().take_while(u8::is_ascii_digit).count();
    let rest = &name[digits..];
    match rest.strip_prefix(['-', '_', ' ']) {
        Some(r) if digits > 0 && !r.is_empty() => (name[..digits].parse().ok(), r),
        _ => (None, name),
    }
}

fn display_name(name: &str) -> &str {
    order_prefix(name).1
}

/// Output directory for a source directory, like clean_dir, except that inside a
/// --files folder names are kept exactly as they are.
fn raw_aware_dir(files_dirs: &[String], dir: &str) -> String {
    for fd in files_dirs {
        if fd.is_empty() {
            return dir.to_string();
        }
        if let Some(rest) = dir.strip_prefix(fd.as_str())
            && (rest.is_empty() || rest.starts_with('/'))
        {
            return format!("{}{rest}", clean_dir(fd));
        }
    }
    clean_dir(dir)
}

/// Output directory for a source directory: ordering prefixes removed from every component.
fn clean_dir(dir: &str) -> String {
    dir.split('/')
        .map(display_name)
        .collect::<Vec<_>>()
        .join("/")
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

// ---------------------------------------------------------------- git

struct GitInfo {
    toplevel: PathBuf,
    /// "Edit this page" URL prefix; the file's path from the repo root is appended.
    edit_base: Option<String>,
    /// Last commit date (YYYY-MM-DD) per path relative to `toplevel`.
    dates: HashMap<String, String>,
}

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "core.quotePath=false"])
        .args(args)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Web URL prefix for editing files, for GitHub and GitLab remotes.
fn edit_url_base(remote: &str, branch: &str) -> Option<String> {
    let r = remote.trim().trim_end_matches('/').trim_end_matches(".git");
    let hostpath = if let Some(rest) = r.strip_prefix("git@") {
        rest.replacen(':', "/", 1)
    } else {
        let rest = r.split_once("://")?.1;
        rest.rsplit_once('@').map_or(rest, |x| x.1).to_string()
    };
    let (host, path) = hostpath.split_once('/')?;
    match host {
        "github.com" => Some(format!("https://github.com/{path}/edit/{branch}/")),
        "gitlab.com" => Some(format!("https://gitlab.com/{path}/-/edit/{branch}/")),
        _ => None,
    }
}

fn git_info(root: &Path) -> Option<GitInfo> {
    let toplevel = PathBuf::from(git(root, &["rev-parse", "--show-toplevel"])?)
        .canonicalize()
        .ok()?;
    let mut branch = git(root, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_default();
    if branch.is_empty() || branch == "HEAD" {
        branch = std::env::var("GITHUB_REF_NAME").unwrap_or_else(|_| "main".into());
    }
    let edit_base =
        git(root, &["remote", "get-url", "origin"]).and_then(|r| edit_url_base(&r, &branch));
    // One `git log` for the whole folder; the first date seen per file is its newest.
    let log = git(root, &["log", "--format=@@%cs", "--name-only", "--", "."]).unwrap_or_default();
    let mut dates = HashMap::new();
    let mut current = String::new();
    for line in log.lines() {
        if let Some(d) = line.strip_prefix("@@") {
            current = d.to_string();
        } else if !line.is_empty() {
            dates
                .entry(line.to_string())
                .or_insert_with(|| current.clone());
        }
    }
    Some(GitInfo {
        toplevel,
        edit_base,
        dates,
    })
}

impl Site {
    /// ("last updated" date, "edit this page" URL) for a source file, when known.
    fn git_meta(&self, key: &str) -> (Option<&str>, Option<String>) {
        let Some(g) = &self.git else {
            return (None, None);
        };
        let Ok(rel) = self
            .root
            .join(key)
            .strip_prefix(&g.toplevel)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
        else {
            return (None, None);
        };
        let date = g.dates.get(&rel).map(String::as_str);
        // Only link files git knows about; an untracked file has no page to edit yet.
        let edit = date
            .and(g.edit_base.as_ref())
            .map(|b| format!("{b}{}", percent_encode_path(&rel)));
        (date, edit)
    }
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
    o.extension.front_matter_delimiter = Some("---".into());
    o.extension.math_dollars = true;
    o.extension.math_code = true;
    o.extension.wikilinks_title_after_pipe = true;
    o.extension.shortcodes = true;
    o.extension.header_id_prefix = id_prefix;
    o.extension.header_id_prefix_in_href = true;
    o.render.r#unsafe = true;
    o.render.github_pre_lang = true;
    o
}

#[derive(Default)]
struct Meta {
    title: Option<String>,
    order: Option<i64>,
    hidden: bool,
}

/// Title (front matter `title:`, else the first `# heading`) plus the optional
/// front matter fields. Front matter is a `---` block of simple `key: value` lines.
fn page_meta(md: &str) -> Meta {
    let arena = Arena::new();
    let doc = parse_document(&arena, md, &md_options(None));
    let mut meta = Meta::default();
    let mut fm_title = None;
    for n in doc.children() {
        if let NodeValue::FrontMatter(fm) = &n.data.borrow().value {
            for line in fm.lines() {
                let Some((k, v)) = line.split_once(':') else {
                    continue;
                };
                let v = v.trim().trim_matches(['"', '\'']).trim();
                match k.trim() {
                    "title" if !v.is_empty() => fm_title = Some(v.to_string()),
                    "order" => meta.order = v.parse().ok(),
                    "hidden" => meta.hidden = v == "true" || v == "yes",
                    _ => {}
                }
            }
        }
    }
    meta.title = fm_title.or_else(|| {
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
    });
    meta
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
        // Each theme applies on its own (OS setting unless the toggle chose one),
        // so neither leaks rules into the other. Uses CSS nesting.
        let theme = |css: String, name: &str, other: &str| {
            format!(
                "@media (prefers-color-scheme: {name}) {{ :root:not([data-theme={other}]) {{\n{css}\n}} }}\n\
                 :root[data-theme={name}] {{\n{css}\n}}\n"
            )
        };
        theme(css(HL_LIGHT), "light", "dark") + &theme(css(HL_DARK), "dark", "light")
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
                    self.warn(format!("{url}: target is excluded from the site"));
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
        if let NodeValue::WikiLink(w) = &mut node.data.borrow_mut().value {
            // [[Page#Some heading]]: the heading part becomes its anchor id.
            let (name, frag) = match w.url.split_once('#') {
                Some((n, f)) => (n.to_string(), Some(comrak::Anchorizer::new().anchorize(f))),
                None => (w.url.clone(), None),
            };
            let target = if name.is_empty() {
                Some(key)
            } else {
                ctx.site.find_wiki(&name)
            };
            match target.and_then(|k| ctx.href(k, false, frag.as_deref())) {
                Some(h) => w.url = h,
                None => ctx.warn(format!("[[{}]]: no page with that name", w.url)),
            }
        }
    }
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
    for (k, is_dir) in site.children(dir) {
        let Some(href) = ctx.href(&k, is_dir, None) else {
            continue;
        };
        if is_dir {
            let name = display_name(file_name(&k));
            h += &format!("<li><a href=\"{}\">{}/</a></li>\n", esc(&href), esc(name));
        } else {
            let e = &site.entries[&k];
            h += &format!(
                "<li><a href=\"{}\">{}</a>{}</li>\n",
                esc(&href),
                esc(&e.title),
                tag(&k, e.kind)
            );
        }
    }
    h + "</ul>\n"
}

/// (level, id, inner html) for the h2/h3 headings in rendered html.
fn toc_entries(html: &str) -> Vec<(u8, String, String)> {
    let mut v = Vec::new();
    let mut rest = html;
    while let Some(i) = rest.find("<h") {
        rest = &rest[i + 2..];
        let level = match rest.as_bytes().first() {
            Some(b'2') => 2,
            Some(b'3') => 3,
            _ => continue,
        };
        let Some(after) = rest[1..].strip_prefix(" id=\"") else {
            continue;
        };
        let Some(q) = after.find('"') else { break };
        let id = &after[..q];
        let Some(gt) = after.find('>') else { break };
        let close = format!("</h{level}>");
        let Some(end) = after.find(&close) else { break };
        let inner = &after[gt + 1..end];
        // Drop the trailing hover anchor, then any remaining tags.
        let inner = inner.find("<a href=\"#").map_or(inner, |a| &inner[..a]);
        let mut text = String::new();
        let mut in_tag = false;
        for c in inner.chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                c if !in_tag => text.push(c),
                _ => {}
            }
        }
        v.push((level, id.to_string(), text.trim().to_string()));
        rest = &after[end..];
    }
    v
}

fn render_toc(html: &str) -> String {
    let entries = toc_entries(html);
    if entries.len() < 2 {
        return String::new();
    }
    let mut h =
        String::from("<aside class=\"toc\">\n<p class=\"toc-title\">On this page</p>\n<ul>\n");
    for (level, id, text) in entries {
        h += &format!(
            "<li class=\"toc-h{level}\"><a href=\"#{}\">{text}</a></li>\n",
            esc(&id)
        );
    }
    h + "</ul>\n</aside>\n"
}

impl Ctx<'_> {
    fn doc_link(&self, d: &Doc) -> Option<(String, String)> {
        let site = self.site;
        match d {
            Doc::Entry(k) => Some((self.href(k, false, None)?, site.entries[k].title.clone())),
            Doc::Listing(dir) => Some((self.href(dir, true, None)?, site.dir_title(dir))),
        }
    }

    /// Previous / next links, following the sidebar order.
    fn pager(&self, order: &[Doc], current: &Doc) -> String {
        let Some(i) = order.iter().position(|d| d == current) else {
            return String::new();
        };
        let link = |d: Option<&Doc>, class: &str, label: &str| {
            d.and_then(|d| self.doc_link(d))
                .map(|(href, title)| {
                    format!(
                        "<a class=\"{class}\" href=\"{}\"><span>{label}</span>{}</a>",
                        esc(&href),
                        esc(&title)
                    )
                })
                .unwrap_or_default()
        };
        let prev = link(
            i.checked_sub(1).and_then(|p| order.get(p)),
            "prev",
            "Previous",
        );
        let next = link(order.get(i + 1), "next", "Next");
        if prev.is_empty() && next.is_empty() {
            return String::new();
        }
        format!("<nav class=\"pager\">{prev}{next}</nav>\n")
    }

    /// "Last updated · Edit this page" line for a source file.
    fn page_meta_line(&self, key: &str) -> String {
        let (date, edit) = self.site.git_meta(key);
        let mut parts = Vec::new();
        if let Some(d) = date {
            parts.push(format!("Last updated {d}"));
        }
        if let Some(e) = edit {
            parts.push(format!("<a href=\"{}\">Edit this page</a>", esc(&e)));
        }
        if parts.is_empty() {
            return String::new();
        }
        format!("<p class=\"page-meta\">{}</p>\n", parts.join(" · "))
    }

    /// Article with its footer (meta line, pager) and table of contents.
    fn wrap(&self, content: &str, key: Option<&str>, order: &[Doc], doc: &Doc) -> String {
        let meta = key.map(|k| self.page_meta_line(k)).unwrap_or_default();
        format!(
            "<div class=\"page-wrap\">\n<article>\n{content}{meta}{}</article>\n{}</div>\n",
            self.pager(order, doc),
            render_toc(content)
        )
    }
}

fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut v = bytes as f64 / 1024.0;
    let mut unit = 0;
    while v >= 1024.0 && unit < UNITS.len() - 1 {
        v /= 1024.0;
        unit += 1;
    }
    format!("{v:.1} {}", UNITS[unit])
}

/// YYYY-MM-DD (UTC) for a timestamp.
fn ymd(t: SystemTime) -> String {
    let days = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / 86_400) as i64;
    // Civil-from-days (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// File list for a --files folder: subfolders, then files with size and date.
fn render_files(ctx: &Ctx, dir: &str) -> String {
    let site = ctx.site;
    let by_name = |a: &&String, b: &&String| {
        file_name(a)
            .to_lowercase()
            .cmp(&file_name(b).to_lowercase())
    };
    let mut subdirs: Vec<&String> = site
        .dirs
        .keys()
        .filter(|d| !d.is_empty() && parent_dir(d) == dir)
        .collect();
    subdirs.sort_by(by_name);
    let mut files: Vec<&String> = site
        .entries
        .keys()
        .filter(|k| parent_dir(k) == dir)
        .collect();
    files.sort_by(by_name);

    let mut rows = String::new();
    let parent = parent_dir(dir);
    if !dir.is_empty()
        && matches!(site.dirs.get(parent), Some(DirIndex::Files(_)))
        && let Some(href) = ctx.href(parent, true, None)
    {
        rows += &format!(
            "<tr><td><a href=\"{}\">../</a></td><td></td><td></td></tr>\n",
            esc(&href)
        );
    }
    for d in &subdirs {
        if let Some(href) = ctx.href(d, true, None) {
            rows += &format!(
                "<tr><td><a href=\"{}\">{}/</a></td><td></td><td></td></tr>\n",
                esc(&href),
                esc(file_name(d))
            );
        }
    }
    let mut total = 0;
    for k in &files {
        let meta = fs::metadata(site.root.join(k)).ok();
        let size = meta.as_ref().map_or(0, |m| m.len());
        total += size;
        let date = match site.git_meta(k).0 {
            Some(d) => d.to_string(),
            None => meta
                .and_then(|m| m.modified().ok())
                .map(ymd)
                .unwrap_or_default(),
        };
        let href = ctx.href(k, false, None).unwrap_or_default();
        rows += &format!(
            "<tr><td><a href=\"{}\">{}</a></td><td>{}</td><td>{date}</td></tr>\n",
            esc(&href),
            esc(file_name(k)),
            human_size(size)
        );
    }
    let summary = match files.len() {
        0 => String::new(),
        1 => format!("1 file, {}", human_size(total)),
        n => format!("{n} files, {}", human_size(total)),
    };
    format!(
        "<h1>{}</h1>\n<p class=\"files-summary\">{summary}</p>\n\
         <table class=\"files\">\n<thead><tr><th>Name</th><th>Size</th><th>Modified</th></tr></thead>\n<tbody>\n{rows}</tbody>\n</table>\n",
        esc(&site.dir_title(dir))
    )
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
    format!(
        "<div class=\"nav-head\">{root}<button class=\"menu-toggle\" aria-label=\"Menu\" aria-expanded=\"false\">☰</button>{THEME_BUTTON}</div>\n{}",
        nav_dir(site, "", &link)
    )
}

fn nav_dir(site: &Site, dir: &str, link: &dyn Fn(&str, bool, &str, &str) -> String) -> String {
    let idx = site.index_key(dir);
    let mut h = String::from("<ul>\n");
    for (k, is_dir) in site.children(dir) {
        if is_dir && matches!(site.dirs.get(&k), Some(DirIndex::Files(_))) {
            let tag = " <span class=\"tag\">files</span>";
            h += &format!(
                "<li>{}</li>\n",
                link(&k, true, display_name(file_name(&k)), tag)
            );
        } else if is_dir {
            h += &format!(
                "<li><details open><summary>{}</summary>\n{}</details></li>\n",
                link(&k, true, display_name(file_name(&k)), ""),
                nav_dir(site, &k, link)
            );
        } else if Some(k.as_str()) != idx {
            let e = &site.entries[&k];
            h += &format!("<li>{}</li>\n", link(&k, false, &e.title, &tag(&k, e.kind)));
        }
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

const THEME_BUTTON: &str = "<button class=\"theme-toggle\" aria-label=\"Toggle dark mode\" title=\"Toggle dark mode\"></button>";
/// Applies a saved light/dark choice before first paint.
const THEME_INIT: &str = "<script>try{const t=localStorage.getItem('webolator-theme');if(t)document.documentElement.dataset.theme=t}catch(e){}</script>";
const KATEX_CSS: &str = "https://cdn.jsdelivr.net/npm/katex@0.16/dist/katex.min.css";
const KATEX_JS: &str = "https://cdn.jsdelivr.net/npm/katex@0.16/dist/katex.min.js";

fn page_html(p: &Page) -> String {
    let nav = match &p.nav {
        Some(n) => format!("<nav class=\"side\">\n{n}</nav>\n"),
        None => format!("<div class=\"corner\">{THEME_BUTTON}</div>\n"),
    };
    let math = if p.body.contains("data-math-style=") {
        format!(
            "<link rel=\"stylesheet\" href=\"{KATEX_CSS}\">\n<script defer src=\"{KATEX_JS}\"></script>\n"
        )
    } else {
        String::new()
    };
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{}</title>\n{THEME_INIT}\n<style>{STYLE}{}</style>\n{math}{}</head>\n<body>\n{nav}<main>\n{}</main>\n<script>{COMMON_JS}</script>\n{}</body>\n</html>\n",
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

/// Loads mermaid; common.js drives rendering (and re-rendering on theme change).
/// Static pages render everything at once; single-file pages render per section.
fn mermaid_script(src: &str, single: bool) -> String {
    let run = if single {
        ""
    } else {
        "<script>webolatorMermaidRun(document)</script>\n"
    };
    format!("{src}\n{run}")
}

// ---------------------------------------------------------------- outputs

struct BuildOpts {
    mermaid_js: Option<PathBuf>,
    css: Option<PathBuf>,
    live_reload: bool,
}

/// User styles: `<root>/webolator.css`, then `--css`. None if there are neither.
fn custom_css(site: &Site, opts: &BuildOpts) -> Result<Option<String>, String> {
    let auto = site.root.join(CUSTOM_CSS);
    let mut parts = Vec::new();
    if auto.is_file() {
        parts.push(read_lossy(&auto));
    }
    if let Some(p) = &opts.css {
        parts.push(fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?);
    }
    Ok((!parts.is_empty()).then(|| parts.join("\n")))
}

const LIVE_RELOAD: &str = "<script>(()=>{let v=null;setInterval(async()=>{try{const t=await (await fetch('/__webolator/version')).text();if(v!==null&&t!==v)location.reload();v=t}catch(e){}},700)})();</script>\n";

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

    write(OUTPUT_MARKER, b"generated by webolator\n")?;
    // Written at the site root so relative url(...) references resolve like in the source.
    let css = custom_css(site, opts)?;
    if let Some(css) = &css {
        write(CUSTOM_CSS, css.as_bytes())?;
    }
    let head_for = |from_out: &str| match &css {
        Some(_) => format!(
            "<link rel=\"stylesheet\" href=\"{}\">\n",
            rel_href(from_out, CUSTOM_CSS)
        ),
        None => String::new(),
    };
    if let Some(js) = &opts.mermaid_js {
        write(
            "_webolator/mermaid.min.js",
            &fs::read(js).map_err(|e| format!("{}: {e}", js.display()))?,
        )?;
    }
    let tail_for = |from_out: &str, body: &str| {
        let mut t = String::new();
        if body.contains("class=\"mermaid\"") {
            let src = match &opts.mermaid_js {
                Some(_) => format!(
                    "<script src=\"{}\"></script>",
                    rel_href(from_out, "_webolator/mermaid.min.js")
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

    let order = site.reading_order();
    let render = |from_out: &str,
                  from_label: &str,
                  title: &str,
                  doc: Doc,
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
        let key = match &doc {
            Doc::Entry(k) => Some(k.as_str()),
            Doc::Listing(_) => None,
        };
        let body = ctx.wrap(&body_fn(&ctx), key, &order, &doc);
        let nav = show_nav(site).then(|| render_nav(&ctx, Some(from_out)));
        let tail = tail_for(from_out, &body);
        let head = head_for(from_out);
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
                head: &head,
                tail: &tail,
            })
            .as_bytes(),
        )?;
        for w in warnings.borrow_mut().drain(..) {
            print_warning(from_label, &w);
        }
        Ok(())
    };

    for (k, e) in &site.entries {
        let out = &site.out[k];
        if e.kind == Kind::Page {
            render(out, k, &e.title, Doc::Entry(k.clone()), &|ctx| {
                render_markdown(ctx, k)
            })?;
        } else {
            write(out, &fs::read(site.root.join(k)).map_err(io)?)?;
        }
    }
    for (d, idx) in &site.dirs {
        match idx {
            DirIndex::Generated => {
                let out = join_key(&clean_dir(d), "index.html");
                render(
                    &out,
                    &out,
                    &site.dir_title(d),
                    Doc::Listing(d.clone()),
                    &|ctx| render_listing(ctx, d),
                )?;
            }
            DirIndex::Files(out) => {
                render(
                    out,
                    out,
                    &site.dir_title(d),
                    Doc::Listing(d.clone()),
                    &|ctx| render_files(ctx, d),
                )?;
            }
            DirIndex::Entry(_) => {}
        }
    }
    let generated = site
        .dirs
        .values()
        .filter(|i| !matches!(i, DirIndex::Entry(_)))
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
    let order = site.reading_order();
    let section = |id: &str, title: &str, doc: Doc, inner: &str| {
        let key = match &doc {
            Doc::Entry(k) => Some(k.clone()),
            Doc::Listing(_) => None,
        };
        let wrapped = ctx_for(id.to_string()).wrap(inner, key.as_deref(), &order, &doc);
        format!(
            "<section class=\"page\" id=\"{}\" data-title=\"{}\" hidden>\n{wrapped}</section>\n",
            esc(id),
            esc(title)
        )
    };
    let flush = |label: &str| {
        for w in warnings.borrow_mut().drain(..) {
            print_warning(label, &w);
        }
    };

    let mut body = String::new();
    for (d, idx) in &site.dirs {
        let id = site.dir_id(d);
        let inner = match idx {
            DirIndex::Generated => render_listing(&ctx_for(id.clone()), d),
            DirIndex::Files(_) => render_files(&ctx_for(id.clone()), d),
            DirIndex::Entry(_) => continue,
        };
        body += &section(&id, &site.dir_title(d), Doc::Listing(d.clone()), &inner);
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
        body += &section(&id, &e.title, Doc::Entry(k.clone()), &inner);
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
        "<script type=\"application/json\" id=\"webolator-assets\">{}</script>\n<script>const WEBOLATOR_FIRST = {};\n{SINGLE_JS}</script>\n",
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
    let head = match custom_css(site, opts)? {
        Some(css) => format!(
            "<style>\n{}\n</style>\n",
            css.replace("</style", "<\\/style")
        ),
        None => String::new(),
    };
    Ok(page_html(&Page {
        title: &site.title,
        nav,
        body: &body,
        head: &head,
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

/// Cheap change detection for --serve: file count and newest mtime, including
/// the ignore files (which the scan itself skips as hidden).
fn fingerprint(input: &Path, scan: &ScanOpts) -> (usize, SystemTime) {
    let mtime = |p: &Path| {
        fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
    };
    let dir = if input.is_file() {
        input.parent().unwrap()
    } else {
        input
    };
    let files = scan_dir(dir, scan).unwrap_or_default();
    let newest = files
        .iter()
        .map(|k| dir.join(k))
        .chain([dir.join(IGNORE_FILE), dir.join(".gitignore")])
        .map(|p| mtime(&p))
        .max()
        .unwrap_or(SystemTime::UNIX_EPOCH);
    (files.len(), newest)
}

fn serve(input: PathBuf, port: u16, opts: BuildOpts, scan: ScanOpts) -> Result<(), String> {
    let tmp = std::env::temp_dir().join(format!("webolator-{}", std::process::id()));
    let watch_input = input.clone();
    let css_path = opts.css.clone();
    let watch_scan = scan.clone();
    let rebuild = {
        let tmp = tmp.clone();
        move || -> Result<(), String> {
            let _ = fs::remove_dir_all(&tmp);
            let site = load_site(&input, &scan)?;
            build_static(&site, &tmp, &opts)?;
            Ok(())
        }
    };
    rebuild()?;
    let version = Arc::new(AtomicU64::new(1));
    {
        let version = version.clone();
        let input = watch_input;
        let css = css_path;
        let state = move || {
            let css_mtime = css
                .as_ref()
                .and_then(|p| fs::metadata(p).and_then(|m| m.modified()).ok());
            (fingerprint(&input, &watch_scan), css_mtime)
        };
        std::thread::spawn(move || {
            let mut last = state();
            loop {
                std::thread::sleep(Duration::from_millis(400));
                let now = state();
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
        if url == "/__webolator/version" {
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
        css: cli.css.clone(),
        live_reload: cli.serve,
    };
    let mut scan = ScanOpts {
        skip: None,
        no_ignore: cli.no_ignore,
        exclude: cli.exclude.clone(),
        git: !cli.no_git,
        files: cli.files.clone(),
    };
    if cli.serve {
        return serve(cli.input, cli.port, opts, scan);
    }
    if cli.single {
        let site = load_site(&cli.input, &scan)?;
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
        scan.skip = Some(out.clone());
        let site = load_site(&cli.input, &scan)?;
        let n = build_static(&site, &out, &opts)?;
        eprintln!(
            "wrote {n} file{} to {}",
            if n == 1 { "" } else { "s" },
            out.display()
        );
    }
    let problems = WARNINGS.load(Ordering::SeqCst);
    if cli.check && problems > 0 {
        return Err(format!(
            "--check: {problems} link problem{} found",
            if problems == 1 { "" } else { "s" }
        ));
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
