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
      --files <DIR>              Show this folder as a file list instead of rendering its contents (repeatable)
      --allow-large              Publish files larger than 10 MB instead of failing
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

## Folders of files

Some folders hold downloads, attachments or assets rather than documentation. Pass `--files` to list such a folder's contents instead of rendering them:

```bash
webolator docs/ --files assets --files downloads/archive
```

- **Nothing inside is rendered.** Markdown files are copied as `.md`, HTML files are served as is, and everything else is copied.
- **The folder and every folder inside it get a file list** with each file's name, size and last-changed date. The date comes from git when available, otherwise from the file's modification time. Each subfolder's list has a `../` link back up.
- **The sidebar shows the folder once,** tagged *files* and linking to the list, rather than every file in it.
- **Links from your pages into the folder** (`[installer](assets/setup.zip)`, or `[assets](assets/)` for the list) work as usual.
- **File names are kept exactly,** including `01-` prefixes, which only order the sidebar outside `--files` folders.
- **If the folder has its own `index.html`,** that file is kept and the list is written to `_files.html` instead.
- **With `--single`,** every file is embedded, so downloads work offline. The output grows by the total size of the files.

The path is relative to the input folder, or to the current directory if it doesn't exist there. `--files .` turns the whole input into a file list. `--files` also works with a single-file input: the folder is included in full, whether or not anything links to it.

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

### Large files

A build stops with an error if any file it would publish is larger than 10 MB, so a stray video or database dump doesn't end up on the site by accident:

```text
error: 1 file over 10.0 MB would be published:
  media/demo.mp4 (48.2 MB)
Leave it out with --exclude '/media/'
(or in .webolatorignore), or pass --allow-large
```

The suggested `--exclude` leaves out the whole folder holding each large file (or just the file, if it's at the top level). Use it as is, put a narrower pattern in `.webolatorignore`, or pass `--allow-large` if it really belongs on the site. `--serve` doesn't check, since it only previews locally.

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
