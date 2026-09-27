# Writing

Everything from GitHub-flavored markdown works: tables, task lists, footnotes, strikethrough, autolinks and `> [!NOTE]` alerts. webolator adds a few extras on top. The [Syntax reference](../syntax.md) lists every form with an example.

## Math

Use `$…$` for inline math, and `$$…$$` or a ```` ```math ```` block for display math. It's rendered with [KaTeX](https://katex.org), which is loaded from a CDN only on pages that contain math.

```markdown
Euler: $e^{i\pi} + 1 = 0$

$$\int_0^1 x^2\,dx = \frac{1}{3}$$
```

Euler: $e^{i\pi} + 1 = 0$

$$\int_0^1 x^2\,dx = \frac{1}{3}$$

## Emoji

Shortcodes like `:rocket:` and `:tada:` become emoji: :rocket: :tada:

## Wikilinks

`[[Page name]]` links to a page by name, as in Obsidian and similar note tools:

| You write | Links to |
|---|---|
| `[[setup]]`, `[[Setup Guide]]` | the page whose file name or title matches |
| `[[guides/advanced]]` | a page by path, without the `.md` |
| `[[setup\|how to install]]` | the same page, with custom link text |
| `[[setup#Step two]]` | a heading on that page |
| `[[#Step two]]` | a heading on the current page |

Matching ignores case and treats spaces, `-` and `_` the same. A name that matches nothing is reported as a warning.

## Front matter

An optional `---` block at the top of a file adjusts that one page. Every field is optional:

```markdown
---
title: Getting started
order: 1
hidden: true
---
```

| Field | Effect |
|---|---|
| `title` | replaces the first `# heading` as the page's title in the sidebar, browser tab and previous/next links |
| `order` | sets the sidebar position and takes precedence over a `01-` filename prefix. On a folder's `README.md` or `index.md`, it positions the whole folder. |
| `hidden` | the page is still built and can be linked to, but it's left out of the sidebar and previous/next links |

The front matter itself is not shown on the page.
