# Site settings

These keys describe the site as a whole. All are optional and go in the `[project]` table of the [config file](README.md).

```toml
[project]
site_name = "Acme Docs"
site_description = "All about Acme."
site_author = "Acme Team"
copyright = "&copy; 2026 Acme"
repo_url = "https://github.com/acme/acme"

[project.theme]
logo = "img/logo.svg"
favicon = "img/favicon.png"
```

## Name and description

`site_name` is the title shown at the top of the sidebar and added to every page's browser title, as in "Setup · Acme Docs". Without it, the name comes from the home page's first `# heading`, or from the folder name when there is no home page.

`site_description` is a short summary. It becomes the tagline on a [generated landing page](04-landing-page.md), and it is the `<meta name="description">` for pages that have no description of their own. Pages with a `description:` in their [front matter](03-front-matter.md), or a first paragraph, use that instead.

`site_author` adds `<meta name="author">` to every page.

## Footer and repository

`copyright` is shown as a small line under every page. It is inserted as HTML, so `&copy;` and links work.

`repo_url` adds a "Repository" button next to "Get started" on a landing page that has no `action:` buttons of its own.

## Logo and favicon

`theme.logo` is a picture shown in the sidebar header, in front of the site name. `theme.favicon` is the browser tab icon. Both are paths relative to the docs folder, and the files must be inside it. A path that doesn't exist is a warning, so [`--check`](../01-usage.md#checking-links-in-ci) catches it.

If you don't set `theme.favicon`, a `favicon.svg`, `favicon.png` or `favicon.ico` in the docs root is picked up automatically.

In `--single` output the logo and favicon are embedded in the file.
