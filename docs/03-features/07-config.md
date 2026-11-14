# Site config and landing page

Everything here is optional. Without any config webolator still builds from the folder alone.

## `webolator.toml` / `zensical.toml`

Put a `webolator.toml` (or a `zensical.toml`) in the docs folder, or in its parent folder. Both file names use the same syntax, so a Zensical project builds as it is. Keys webolator doesn't know are ignored.

```toml
[project]
site_name = "Acme Docs"
site_description = "All about Acme."
site_url = "https://example.com/acme/"
site_author = "Acme Team"
copyright = "&copy; 2026 Acme"
repo_url = "https://github.com/acme/acme"
docs_dir = "docs"          # only when you pass the project folder

nav = [
  "index.md",
  { "Getting started" = ["install.md", { "First steps" = "first.md" }] },
  { "GitHub" = "https://github.com/acme/acme" },
]

[project.theme]
logo = "img/logo.svg"      # paths are relative to the docs folder
favicon = "img/favicon.png"
```

| Key | Effect |
|---|---|
| `site_name` | Site title in the sidebar and in every page `<title>` |
| `site_description` | Home page tagline and the default `<meta name="description">` |
| `site_url` | Canonical links, OpenGraph URLs, `sitemap.xml`, `robots.txt`, and absolute links on the 404 page |
| `site_author` | `<meta name="author">` |
| `copyright` | Footer line (HTML allowed) |
| `repo_url` | "Repository" button on the home page |
| `docs_dir` | Pages folder, when you build from the project folder (`zensical.toml` assumes `docs/`) |
| `nav` | Explicit sidebar and previous/next order. Pages not listed are still built |
| `theme.logo`, `theme.favicon` | Logo in the sidebar header and the tab icon. A `favicon.svg`, `.png` or `.ico` in the docs root is picked up automatically |

## Landing page

If there is no `index.md` or `README.md`, the home page is generated: a hero with the site name and description, and a card for every top-level page and section. Each card shows the page's description, which is its first paragraph, or `description:` from the front matter.

To design your own, give `index.md` the home layout:

```markdown
---
layout: home
tagline: Zero-config docs from markdown.
action: Get started | install.md
action2: GitHub | https://github.com/acme/acme
---
# Acme

Any other markdown goes below the hero, followed by the section cards.
```

The first `# heading` becomes the hero title. The tagline is `tagline:`, then `description:`, then the first paragraph. Without `action:` lines you get a "Get started" button to the first page, plus a "Repository" button when `repo_url` is set.

## Deployment extras

Every static build also writes:

- `404.html` (unless you have your own `404.md`), which GitHub Pages and Netlify serve for missing pages. `--serve` does too.
- `sitemap.xml` and `robots.txt`, when `site_url` is set.
- A page description, canonical URL and OpenGraph tags in every page's `<head>`.
