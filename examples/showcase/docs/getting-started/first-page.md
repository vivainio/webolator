---
description: Write a markdown file, run one command, get a website.
---
# Your first page

Create `docs/hello.md`:

```markdown title="docs/hello.md"
# Hello

This is my first page.
```

Run `webolator docs/ --serve` and open the printed address. The page appears in the sidebar, with its title taken from the heading.

??? tip "Where do descriptions come from?"
    The summary under each card on the home page is the page's `description:` front matter, or else its first paragraph. This page sets one explicitly.

Next: [admonitions](../guide/admonitions.md).
