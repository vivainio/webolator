# Landing page

The home page is the first thing a reader sees. There are three ways to get one.

## 1. Write your own `index.md`

With an `index.md` or `README.md` in the docs root, that file is the home page, written like any other page.

## 2. A generated landing page

Without a home page file, webolator makes one: a **hero** with the site name, the `site_description` and a "Get started" button, then a **card** for every top-level page and section. Each card shows the page's description, which is its front matter `description:` or its first paragraph. A section's card shows how many pages it holds, or its index page's description.

The "Get started" button goes to the first page in the reading order. If `repo_url` is set, a "Repository" button follows it. With an explicit [`nav`](02-navigation.md), the cards follow the nav instead of the folders.

## 3. A designed landing page: `layout: home`

Give your own index page the home layout to keep your content and add the hero and cards:

```markdown
---
layout: home
tagline: A sample documentation site, built from plain markdown.
action: Get started | getting-started/install.md
action2: GitHub | https://github.com/acme/acme
---
# Acme Docs

Any other markdown goes here, below the hero and above the cards.
```

- The first `# heading` becomes the hero title.
- The tagline is `tagline:`, then `description:`, then the page's first paragraph, which is moved into the hero.
- `action:` and `action2:` are written `Label | link`. The link can be a page in the site, which is checked like any other link, or a full URL. The first button is the highlighted one.
- Without any `action:` lines you get the default "Get started" and "Repository" buttons.
- The cards come after your markdown and list what's in the folder the page is the index of.

Every part can be restyled with [CSS](../03-features/04-styling.md): the hero is a `header.hero` with a `.tagline` and `.actions`, and the cards are `.cards` with `a.card` inside.

For a working example, see the [showcase site](https://vivainio.github.io/webolator/showcase/) and its [source](https://github.com/vivainio/webolator/tree/main/examples/showcase).
