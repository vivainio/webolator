# Deployment

A few settings make a site ready for the web. Most of them depend on one: the address where it will be published.

```toml
[project]
site_url = "https://example.com/acme/"
```

## What `site_url` turns on

With `site_url` set, a static build also writes:

- **`sitemap.xml`** with the address of every page and generated index page, except pages marked `hidden`.
- **`robots.txt`** that allows everything and points to the sitemap. If you provide your own `robots.txt`, yours is kept.
- **Canonical links and OpenGraph addresses** (`<link rel="canonical">` and `og:url`) in every page, so search engines and link previews agree on one address per page.
- **Working links on the 404 page** (see below).

Include the folder if the site isn't at the root of its domain: a GitHub Pages project site is `https://user.github.io/project`.

## Always on

These need no setting:

- **Descriptions and previews.** Every page gets `<meta name="description">`, `og:title`, `og:description`, `og:site_name` and `og:type`. The description is the page's front matter `description:`, then its first paragraph, then the [`site_description`](01-site-settings.md). `site_author` adds `<meta name="author">`.
- **`404.html`.** Hosts such as GitHub Pages and Netlify show it for missing pages, and `--serve` does too. Without `site_url`, its links are relative, so they only work from the root. With `site_url`, they work at any depth. To write your own, add a `404.md`, which replaces the generated one.
- **Icons.** A `favicon.*` in the docs root is picked up. See [Site settings](01-site-settings.md#logo-and-favicon).

## Checking before you ship

```bash
webolator docs/ -o _site --check
```

`--check` exits with an error if any link is broken, a file named in the config is missing, or a nav entry doesn't exist. Run it in CI so a bad link never reaches the published site. [Publishing to GitHub Pages](../05-publishing.md) has a ready-made workflow.
