---
name: webolator
description: Turn a folder of markdown, or a single .md file, into a browsable website or one self-contained HTML file with the webolator CLI. Use when the user wants to publish, preview, share or check markdown docs, or asks about webolator config, navigation, search, diagrams, tabs, galleries or GitHub Pages deployment.
---

# webolator

Zero-config markdown to website. Point it at a folder: the sidebar comes from the directory tree and page titles from each file's first `# heading`. No index file or `SUMMARY.md` is needed.

## Commands

```bash
webolator docs/                  # static site in ./site
webolator docs/ -o public        # ...or somewhere else
webolator docs/ --serve          # http://127.0.0.1:8000 with live reload (--port N)
webolator docs/ --single         # one self-contained docs.html, easy to email
webolator notes.md --single      # one file plus everything it links to
webolator docs/ --check          # exit 1 on broken links (use in CI)
```

Useful flags:

- `--exclude GLOB` (repeatable), `--no-ignore`: what to leave out. `.gitignore` and `.webolatorignore` are honored.
- `--files DIR`: show a folder as a file list (name, size, date) instead of rendering it.
- `--gallery DIR`: show a folder of images as a thumbnail grid with click-to-zoom.
- `--css FILE`, `--js FILE`: extra stylesheet or script for every page. A `webolator.css` or `webolator.js` in the docs root is picked up automatically.
- `--mermaid-js FILE`: use a local mermaid.min.js instead of the CDN (offline use).
- `--no-git`: skip "last updated" and "Edit this page" from git.
- `--allow-large`: publish files over 10 MB.

When changing or writing docs, run `--check` and fix every warning it prints. Run `--serve` to look at the result.

## How the site is put together

- `index.md` or `README.md` in a folder becomes that folder's `index.html`. A folder without one gets a generated page; at the root it is a landing page with a hero and section cards.
- Number prefixes (`01-intro.md`, `02-guides/`) set the sidebar order and are removed from labels and URLs.
- Front matter (all optional): `title`, `order`, `hidden: true`, `description`, and `layout: home` on an index page for a hero landing page (`tagline`, `action: Label | link`, `action2`).
- Links between files work as written: `other.md`, `dir/`, `../x.md#heading`, `/root/relative.md`, `[[Wikilinks]]`. Broken links are warnings.
- Other files (images, PDFs, HTML, text) are copied and linked.

## Markdown extras

GitHub-flavored markdown plus alerts (`> [!NOTE]`), footnotes, `$math$`, `:emoji:`, and:

- Mermaid: a fenced block with the language `mermaid`.
- Tabs: `=== "Title"` followed by a body indented four spaces. Consecutive tabs form a group.
- Admonitions: `!!! warning "Title"` (collapsible: `??? tip`, open by default: `???+`), body indented four spaces.
- Snippets: a line `--8<-- "path/to/file"` includes that file.
- Code titles: a fence with `title="app.py"` after the language.
- Galleries: a fence with the language `gallery`, listing a folder, files or a glob such as `shots/*.png`, one per line.

## Config (optional)

A `webolator.toml` or `zensical.toml` in the docs folder (or its parent) with a `[project]` table. The same syntax works in both, so a Zensical project builds as it is:

```toml
[project]
site_name = "Acme Docs"
site_description = "All about Acme."
site_url = "https://example.com/acme/"   # enables sitemap.xml, robots.txt, canonical links
site_author = "Acme Team"
copyright = "&copy; 2026 Acme"
repo_url = "https://github.com/acme/acme"
search = false                           # drop the search box and index
extra_css = ["css/brand.css"]            # local files or URLs
extra_javascript = ["js/app.js", { path = "js/m.js", type = "module" }]
nav = [
  "index.md",
  { "Getting started" = ["install.md", { "First steps" = "first.md" }] },
  { "GitHub" = "https://github.com/acme/acme" },
]

[project.theme]
logo = "img/logo.svg"
favicon = "img/favicon.png"
```

Without `nav` the sidebar follows the directory tree. Unknown keys are ignored.

## Built-in extras

Every static build has a search box (press `/`), a light/dark toggle, an "On this page" sidebar, previous/next links, `404.html`, and OpenGraph/description meta tags. Search is a small word index loaded on first use, with prefix matching and no typo tolerance.

## Publishing to GitHub Pages

Run `uvx webolator docs -o _site --check` in a workflow, then upload `_site` with `actions/upload-pages-artifact` and deploy with `actions/deploy-pages`. Check out with `fetch-depth: 0` so per-page "last updated" dates work.

## Reference

Docs: https://vivainio.github.io/webolator/ (syntax reference, config, extensions, publishing). Sample: https://vivainio.github.io/webolator/showcase/
