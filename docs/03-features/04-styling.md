# Custom styling

There are two ways to add your own CSS. Both are applied after the built-in styles, so your rules win.

1. **`webolator.css` in the root of your docs folder** is picked up automatically, so a styled site still needs no flags. It isn't listed as a page.
2. **`--css path/to/file.css`** adds a stylesheet from anywhere. If both exist, `webolator.css` comes first and `--css` after it.

In a static build the combined CSS is written to `webolator.css` at the site root, so relative `url(...)` references work just as they do in your source folder. With `--single` it is inlined into the file, where relative `url(...)` references won't resolve, so use absolute URLs or `data:` URIs there. `--serve` reloads the page when either file changes.

## Colors

All colors are CSS variables. Overriding them is usually all a theme needs:

```css
:root {
  --link: #c2185b;
  --side: #fff3e0;
}
```

| Variable | Used for |
|---|---|
| `--bg`, `--fg` | page background and text |
| `--muted` | secondary text: blockquotes, footnotes, tags |
| `--border` | rules, table borders, heading underlines |
| `--code-bg` | code blocks and inline code |
| `--link` | links and the active sidebar item |
| `--side`, `--hover`, `--active` | sidebar background, hover and current page |
| `--note`, `--tip`, `--important`, `--warning`, `--caution` | `> [!NOTE]`-style alerts |

The built-in dark theme sets these variables inside `@media (prefers-color-scheme: dark)`. A plain `:root` rule of yours overrides both themes. To change only one, wrap your rule in the same media query.

## Page structure

```html
<body>
  <nav class="side">                      <!-- sidebar; missing when there's only one page -->
    <a data-site href="…">Site title</a>
    <ul>
      <li><a href="…" class="active">Page</a></li>
      <li><details open><summary><a href="…">folder</a></summary><ul>…</ul></details></li>
    </ul>
  </nav>
  <main>
    <article> … your markdown … </article>
  </main>
</body>
```

Inside `article` you get plain HTML (`h1`–`h6`, `p`, `table`, `blockquote`, `pre > code`), plus a few classes:

| Selector | What it is |
|---|---|
| `pre.mermaid` | a mermaid diagram |
| `pre.syntax-highlighting`, `.hl-*` | highlighted code and its tokens (`.hl-keyword`, `.hl-comment`, …) |
| `.markdown-alert`, `.markdown-alert-note`, … | GitHub-style alerts |
| `.footnotes` | the footnote list |
| `.anchor` | the `#` link shown when you hover over a heading |
| `.tag` | the small file-type label next to non-markdown files in the sidebar |
| `ul.listing` | the file list on a generated folder page |
| `section.page` | one page, in `--single` output only |

## Examples

A narrower serif reading column:

```css
article { max-width: 70ch; font-family: Georgia, serif; font-size: 18px; }
```

Hide the sidebar:

```css
nav.side { display: none; }
```
