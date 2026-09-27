# Reading

Every page gets these without any setup.

## On this page

A right-hand sidebar lists the page's `##` and `###` headings and highlights the one you're reading. It appears on pages with at least two such headings, and it's hidden on narrow screens.

## Previous and next

At the bottom of each page are links to the previous and next page, in sidebar order. That means the [ordering](../01-usage.md#ordering) you set with `01-` prefixes or front matter also sets the reading order. Pages marked `hidden: true` are skipped.

## Last updated and "Edit this page"

When the docs are in a git repository, each page shows the date of the last commit that changed it. When the `origin` remote is on GitHub or GitLab, it also shows a link to edit the file there. Files not yet committed get neither. Pass `--no-git` to leave both out.

In CI, fetch the full history (`fetch-depth: 0` in `actions/checkout`). Otherwise every page shows the date of the latest commit.

## Light and dark

Pages follow the operating system's setting. The ☾/☀ button in the sidebar switches themes, and the choice is remembered in the browser. Mermaid diagrams and code highlighting switch too.

## Code blocks

Hover over a code block to show a **Copy** button.

## Images

Click an image to see it full size. Click again or press Esc to close it. Images that are links behave as links.

## Phones

On narrow screens the sidebar collapses into a ☰ menu at the top.
