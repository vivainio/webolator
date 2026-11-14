# Front matter

An optional block at the very top of a page adjusts that one page. It is a `---` line, simple `key: value` lines, and another `---` line:

```markdown
---
title: Getting started
order: 1
description: Install it and build your first site.
---
# Welcome

The page starts here.
```

The block is not shown on the page. Values can be wrapped in single or double quotes, and each key has to fit on one line (there are no lists or nested keys).

## Fields

| Field | Effect |
|---|---|
| `title` | Replaces the first `# heading` as the page's title in the sidebar, the browser tab, previous/next links and search results |
| `order` | A number that sets the sidebar position and wins over a `01-` filename prefix. On a folder's `README.md` or `index.md`, it positions the whole folder |
| `hidden` | `true` or `yes`: the page is built and can be linked to, but it's left out of the sidebar, previous/next links, search and the sitemap |
| `description` | A one-line summary, used as the page's `<meta name="description">`, in link previews, in the cards of a landing page and in folder listings. Without it the first paragraph is used, shortened to about 180 characters |
| `layout` | `home` turns an index page into a [landing page](04-landing-page.md) |
| `tagline` | Landing page only: the line under the title. It defaults to `description`, then the first paragraph |
| `action`, `action2` | Landing page only: buttons, written `Label \| link`, where the link is a page, an anchor or a full URL |

Other keys are accepted and ignored, so front matter written for another tool does no harm.

## An example landing page

```markdown
---
layout: home
tagline: Zero-config docs from markdown.
action: Get started | install.md
action2: GitHub | https://github.com/acme/acme
---
# Acme
```

See [Landing page](04-landing-page.md) for what this produces.
