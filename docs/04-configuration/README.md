# Configuration

webolator needs no configuration: point it at a folder and it builds. This chapter is for when you want more control. There are four places to put settings, from widest to narrowest:

| Where | What it controls | Chapter page |
|---|---|---|
| A config file, `webolator.toml` or `zensical.toml` | The whole site: name, URL, logo, navigation, extra CSS and JS, search | this page and the ones below |
| Front matter at the top of a page | One page: title, order, visibility, description, landing-page layout | [Front matter](03-front-matter.md) |
| Command-line flags | One run: output folder, `--single`, `--serve`, `--check`, `--css`, `--js` | [Usage](../01-usage.md) |
| Files and folder names | `01-` prefixes for order, `.webolatorignore` for exclusions, `webolator.css` / `webolator.js` for styles and scripts | [Usage](../01-usage.md), [Styles and scripts](06-styles-and-scripts.md) |

## The config file

Put a `webolator.toml` or a `zensical.toml` in the docs folder, or in its parent folder (the usual place in a Zensical project, whose pages are in `docs/`). Both names use the same syntax, so a Zensical project builds as it is.

- **Which file wins.** webolator looks in the docs folder first, then in its parent, and in each place prefers `webolator.toml` over `zensical.toml`. Only one file is read.
- **Building from the project folder.** If you pass the folder that holds the config, pages are read from its `docs_dir`. A `zensical.toml` without `docs_dir` assumes `docs/` when that folder exists, as Zensical does.
- **Not published.** The config file never becomes a page or a download.
- **Syntax.** Settings go in a `[project]` table. Keys at the top of the file are accepted too when there is no `[project]` table. A syntax error stops the build and names the file.
- **Unknown keys are ignored**, so options for other tools can stay in the file.

```toml
[project]
site_name = "Acme Docs"
site_description = "All about Acme."
site_url = "https://example.com/acme/"
site_author = "Acme Team"
copyright = "&copy; 2026 Acme"
repo_url = "https://github.com/acme/acme"
docs_dir = "docs"          # only when you pass the project folder
search = false             # leave out the search box and its index

nav = [
  "index.md",
  { "Getting started" = ["install.md", { "First steps" = "first.md" }] },
  { "GitHub" = "https://github.com/acme/acme" },
]

extra_css = ["css/brand.css"]
extra_javascript = ["js/app.js", { path = "js/m.js", type = "module" }]

[project.theme]
logo = "img/logo.svg"      # paths are relative to the docs folder
favicon = "img/favicon.png"
```

## All settings

| Key | Effect | Details |
|---|---|---|
| `site_name` | Site title in the sidebar and in every page `<title>` | [Site settings](01-site-settings.md) |
| `site_description` | Home page tagline and the default `<meta name="description">` | [Site settings](01-site-settings.md) |
| `site_author` | `<meta name="author">` | [Site settings](01-site-settings.md) |
| `copyright` | Footer line under every page (HTML allowed) | [Site settings](01-site-settings.md) |
| `repo_url` | "Repository" button on the home page | [Site settings](01-site-settings.md) |
| `theme.logo`, `theme.favicon` | Logo in the sidebar header, and the tab icon | [Site settings](01-site-settings.md) |
| `site_url` | Canonical links, `sitemap.xml`, `robots.txt`, a working 404 page | [Deployment](07-deployment.md) |
| `nav` | Explicit sidebar and previous/next order | [Navigation](02-navigation.md) |
| `search` | `false` removes the search box and its index | [Search](05-search.md) |
| `extra_css`, `extra_javascript` | Extra stylesheets and scripts on every page | [Styles and scripts](06-styles-and-scripts.md) |
| `docs_dir` | Pages folder, when you build from the project folder | above |

## Coming from Zensical or MkDocs

The keys above are the same as in Zensical, so most `zensical.toml` files work unchanged. What webolator doesn't read:

- `markdown_extensions` and other Python-Markdown settings. webolator has its own markdown engine, and the popular extensions (tabs, admonitions, snippets, code titles) are built in. See [Tabs, admonitions, snippets and galleries](../03-features/08-extensions.md).
- Plugins and theme options other than `logo` and `favicon`. Styling is done with [CSS variables](../03-features/04-styling.md).
- Version switchers, i18n and blog settings.

A `mkdocs.yml` is not read. Move its settings into a `[project]` table. Its `nav` list translates closely, because TOML arrays and inline tables take the same shape.
