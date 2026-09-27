# Usage

```text
webolator [OPTIONS] <INPUT>

Arguments:
  <INPUT>  Markdown file or directory

Options:
  -o, --out <OUT>                Output directory (default ./site), or output file with --single (default <name>.html)
      --single                   Produce one self-contained HTML file
      --serve                    Serve locally with live reload instead of writing output
      --port <PORT>              [default: 8000]
      --mermaid-js <MERMAID_JS>  Use this local mermaid.min.js (inlined/copied) instead of the CDN
      --css <CSS>                Extra stylesheet applied after the built-in styles (in addition to <root>/webolator.css)
```

## A folder

```bash
webolator docs/              # writes ./site
webolator docs/ -o public    # writes ./public
```

Every markdown file becomes a page. Each folder gets an `index.html`:

| Source | Result |
|---|---|
| `index.md` or `README.md` in a folder | that folder's `index.html` |
| a folder with no index | a generated listing page |
| `foo.md` | `foo.html` (or `foo.md.html` if a raw `foo.html` already exists) |
| any other file | copied as is |
| hidden files and folders, `node_modules`, `target`, `venv` | skipped |

## Ordering

The sidebar is sorted by name, with files before folders. To choose the order, give files and folders a number prefix such as `01-usage.md` or `02-guides/`. Numbered items come first, in numeric order. The prefix doesn't appear in labels, and it's removed from output paths too, so `02-guides/01-setup.md` becomes `guides/setup.html`. Links in your markdown still use the real file names.

The site title is the first heading of the root `README.md` or `index.md`. If there is neither, the folder name is used.

## A single file

```bash
webolator notes.md
```

webolator follows the file's local links, including links into parent folders, and builds a site from everything it reaches. The file you passed becomes the home page.

## Preview

```bash
webolator docs/ --serve --port 8000
```

This serves the site on `http://127.0.0.1:8000/`. The site is rebuilt whenever a file changes, and open pages reload themselves.

## Single HTML file

```bash
webolator docs/ --single -o docs.html
```

See [Single-file output](03-features/03-single-file.md).
