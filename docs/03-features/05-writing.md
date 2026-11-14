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

An optional `---` block at the top of a file adjusts that one page:

```markdown
---
title: Getting started
order: 1
hidden: true
---
```

`title`, `order` and `hidden` change how the page appears in the sidebar, and `description` and `layout` control link previews and landing pages. Every field is optional, and the block is not shown on the page. See [Front matter](../04-configuration/03-front-matter.md) for the full list.
