# webolator

Turn a folder of markdown, or a single `.md` file, into a browsable website. No config file, no index file, no `SUMMARY.md`.

- **Zero config.** Point it at a folder and the navigation comes from the directory tree. Page titles come from the first `# heading`.
- **Mermaid.** ```` ```mermaid ```` blocks become diagrams, rendered in the browser.
- **Syntax highlighting** happens at build time (syntect), with light and dark themes and no JavaScript.
- **Cross-file links work.** `other.md`, `dir/`, `../x.md#some-heading` and `/root/relative.md` are rewritten to the right pages and anchors. Broken links print a warning.
- **Sibling files are kept.** Raw `.html`, `.txt`/`.log`/code files, PDFs and images are copied and linked. HTML, text and PDF files also appear in the sidebar.
- **`--single`** writes one self-contained `.html` with everything embedded (images, text files, HTML files, PDFs), which is easy to email or attach.
- **`--serve`** runs a local preview with live reload.
- GitHub-flavored markdown: tables, task lists, footnotes, strikethrough, autolinks and `> [!NOTE]` alerts.
- Light and dark mode follow the OS setting.

## Install

```bash
cargo install --git https://github.com/vivainio/webolator
```

## Usage

```bash
webolator docs/                     # static site in ./site
webolator docs/ -o public           # ...or somewhere else
webolator docs/ --single            # one self-contained docs.html
webolator notes.md --single         # a single file, plus anything it links to
webolator docs/ --serve             # http://127.0.0.1:8000 with live reload
```

Try it on the bundled demo:

```bash
cargo run -- examples/demo --serve
```

### How the site is put together

| Source | Result |
|---|---|
| `index.md` or `README.md` in a folder | that folder's `index.html` |
| folder with no index | a generated listing page |
| `foo.md` | `foo.html` (or `foo.md.html` if a raw `foo.html` already exists) |
| any other file | copied as is |
| hidden files and folders, `node_modules`, `target`, `venv` | skipped |

If you pass a **single file**, webolator follows its local links, including links to parent folders. The site then contains that file, which becomes the home page, plus everything reachable from it.

In **`--single` mode**, each page becomes a section with hash routing, so the back and forward buttons and deep links like `docs.html#p-guide-install--setup` work. Images are inlined. Linked HTML files are shown in an iframe and text files as preformatted text, and PDFs and other files open from embedded blobs.

### Mermaid offline

By default mermaid.js is loaded from the jsDelivr CDN, and only on pages that contain a diagram. To work fully offline, pass a local copy:

```bash
webolator docs/ --single --mermaid-js mermaid.min.js
```

In static mode the file is copied to `_webolator/`. In `--single` mode it is inlined.
