# Links

You write links the same way you would for GitHub, and webolator rewrites them to point at the generated pages.

| You write | Static site gets | Single file gets |
|---|---|---|
| `[x](other.md)` | `other.html` | `#p-other` |
| `[x](guide/)` | `guide/index.html` | the guide's index section |
| `[x](../README.md#install)` | `../index.html#install` | `#p-readme--install` |
| `[x](/guide/setup.md)` (root-relative) | the relative path to `guide/setup.html` | `#p-guide-setup` |
| `[x](#section)` | unchanged | prefixed with the page id |
| `[x](report.html)`, `[x](build.log)` | copied and linked as is | shown inside the page |
| `[x](spec.pdf)` | copied and linked as is | opened from embedded data |
| `https://…`, `mailto:…` | unchanged | unchanged |

Links written as raw HTML inside markdown, such as `<img src="…">` and `<a href="…">`, are rewritten too.

## Heading anchors

Heading ids follow GitHub's rules, so a `#some-heading` anchor that works on GitHub also works here. Hover over a heading to see its `#` link.

## Broken links

A link to a file that doesn't exist is left alone and reported during the build:

```text
warning: README.md: broken link: nope.md
```

## Folders that work from disk

Links to a folder always point at its `index.html` explicitly. That way a static build also works when opened directly from disk (`file://`), without a web server.
