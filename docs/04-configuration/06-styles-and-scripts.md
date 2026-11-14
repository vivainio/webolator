# Styles and scripts

Add your own CSS and JavaScript to every page. For what to put in a stylesheet (colors, layout, examples), see [custom styling](../03-features/04-styling.md).

## Files with a fixed name

| File | Effect |
|---|---|
| `webolator.css` in the docs root | Applied to every page, after the built-in styles |
| `webolator.js` in the docs root | Added to every page, at the end |

Neither appears in the sidebar. The same files can be passed from the command line, which adds them after the ones in the folder:

```bash
webolator docs/ --css brand.css --js analytics.js
```

## `extra_css` and `extra_javascript`

For more than one file, or for files from a CDN, use these keys in the [config file](README.md). They are the same as in Zensical and MkDocs:

```toml
[project]
extra_css = ["css/brand.css", "https://fonts.example.com/x.css"]
extra_javascript = [
  "js/analytics.js",
  "https://cdn.example.com/lib.js",
  { path = "js/app.js", type = "module", defer = true },
]
```

- **Local paths** are relative to the docs folder. The files are copied to the same place in the site, so `url(...)` in a stylesheet resolves like in the source. They don't appear in the sidebar or in search.
- **URLs** are linked as they are.
- **The table form** takes `type`, `async` and `defer`.
- **A path that doesn't exist** is a warning, so `--check` catches it.

## Order

Stylesheets go in the page head: the built-in styles, then `extra_css` in the order listed, then `webolator.css`. Scripts go at the end of the page: the built-in script, then `extra_javascript` in the order listed, then `webolator.js`.

## Single-file output

With `--single`, local files are inlined into the one HTML file, so it stays self-contained. URLs stay as `<link>` and `<script>` tags, so those need a network connection. Relative `url(...)` references inside an inlined stylesheet can't be resolved.

Scripts run with full access to the page, so only add code you trust.
