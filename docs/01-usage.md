# Usage

If webolator isn't installed yet, see the [installation instructions](https://github.com/vivainio/webolator#installation).

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
      --exclude <GLOB>           Leave out files matching this gitignore-style pattern (repeatable)
      --no-ignore                Include files that .gitignore / .ignore would leave out (.webolatorignore still applies)
      --no-git                   Don't add "last updated" dates and "edit this page" links from git
      --check                    Fail (exit code 1) if there are broken or excluded links
  -h, --help                     Print help
  -V, --version                  Print version
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
| anything matched by an ignore rule | skipped, see [Leaving files out](#leaving-files-out) |

## Leaving files out

Three kinds of rules keep files out of the site. All of them use gitignore syntax, including `**` and `!` negation.

1. **`.gitignore` files are honored by default.** This includes nested ones and those in parent folders, and it works even when the folder isn't a git repository. Build output and scratch files usually stay out without any setup. `.ignore` files work the same way. Pass `--no-ignore` to turn this off.
2. **`.webolatorignore`** is for files that belong in git but not on the site. It works like a `.gitignore` and can be placed in any folder:

   ```gitignore
   drafts/
   internal-*.md
   !internal-overview.md
   ```

   `--no-ignore` doesn't affect it.
3. **`--exclude <GLOB>`** works for one-off runs. It can be repeated, and patterns are relative to the input folder:

   ```bash
   webolator docs/ --exclude 'scratch/' --exclude '*.log'
   ```

A link to an excluded file stays unchanged and is reported, so you'll notice if something you excluded is still linked:

```text
warning: README.md: drafts/plan.md: target is excluded from the site
```

With `--serve`, excluded files aren't watched, and editing `.webolatorignore` or `.gitignore` rebuilds the site.

When the input is a single file, the site contains only that file and what it links to, so the ignore rules aren't needed there and aren't applied.

## Ordering

The sidebar is sorted by name, with files before folders. To choose the order, give files and folders a number prefix such as `01-usage.md` or `02-guides/`. Numbered items come first, in numeric order. The prefix doesn't appear in labels, and it's removed from output paths too, so `02-guides/01-setup.md` becomes `guides/setup.html`. Links in your markdown still use the real file names.

A page can also set its position with `order:` in [front matter](03-features/05-writing.md#front-matter), which takes precedence over the prefix. The same order is used for the previous and next links at the bottom of each page.

## Checking links in CI

```bash
webolator docs/ --check
```

`--check` builds as usual, but exits with code 1 if there were any broken links, links to excluded files or unknown `[[wikilinks]]`. Use it to keep docs from rotting.

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
