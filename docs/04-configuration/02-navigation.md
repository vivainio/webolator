# Navigation

By default the sidebar follows the folder tree and needs no setup. When you need a different shape, give the [config file](README.md) a `nav` list.

## The default: folders and files

- Folders become sections, and `index.md` or `README.md` in a folder is that section's own page.
- Order is by number prefix (`01-intro.md`, `02-guides/`), then files before folders, then name. Prefixes are hidden from labels and URLs.
- A page's [front matter](03-front-matter.md) can change its position with `order:`, or remove it from the sidebar with `hidden: true`.
- Labels come from each page's title, which is its front matter `title:`, or else its first `# heading`.

This is enough for most sites.

## An explicit nav

```toml
[project]
nav = [
  "index.md",
  { "Getting started" = [
    "install.md",
    { "First steps" = "first.md" },
  ] },
  { "Reference" = "reference/api.md" },
  { "GitHub" = "https://github.com/acme/acme" },
]
```

There are four kinds of entry:

| Entry | Result |
|---|---|
| `"page.md"` | A link to that page, labelled with its title |
| `{ "Label" = "page.md" }` | The same, with your own label |
| `{ "Section" = [ ... ] }` | A section that groups other entries. It can nest, and it has no page of its own |
| `{ "Label" = "https://..." }` | A link to another site, shown with a ↗ |

How it behaves:

- **Paths** are relative to the docs folder, and use the real file name, including any `01-` prefix.
- **Labels** replace the page title in the sidebar, the previous/next links and the browser title.
- **Previous and next** follow the nav order. The home page comes first, unless you place it in the list yourself.
- **Pages you don't list** are still built and can be linked to and found by search. They just aren't in the sidebar or the previous/next order.
- **A path that doesn't exist** is a warning, and the entry is dropped. `--check` turns warnings into a failed build.
- **Landing page cards** follow the nav too, with one card per top-level entry. A section's card links to its first page.
- **Number prefixes and `order:`** no longer matter for the sidebar once `nav` is set.

The syntax is the same as `nav` in `zensical.toml`, so an existing Zensical nav works as it is.
