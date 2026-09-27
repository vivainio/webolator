# Publishing to GitHub Pages

This site is published by the following workflow ([`.github/workflows/pages.yml`](https://github.com/vivainio/webolator/blob/main/.github/workflows/pages.yml)). To use the same approach in your own repository:

1. In **Settings → Pages**, set **Source** to **GitHub Actions**.
2. Add this workflow:

```yaml
name: Docs

on:
  push:
    branches: [main]
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: true

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: astral-sh/setup-uv@v6
      - run: uvx webolator docs -o _site
      - uses: actions/upload-pages-artifact@v4
        with:
          path: _site

  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - id: deployment
        uses: actions/deploy-pages@v4
```

webolator's own workflow builds webolator from source instead of running `uvx`, so the docs always match the current code.
