# Mermaid diagrams

Click a diagram to view it larger. Drag to pan; use the + and − buttons, mouse wheel, or a pinch gesture to zoom. Reset restores the original view, and Close or Escape returns to the page.

Try it with this diagram:

```mermaid
flowchart LR
  A[Write Markdown] --> B[Render Mermaid]
  B --> C[Click the diagram]
  C --> D[Zoom and pan]
  D --> E[Close the preview]
```

A fenced code block whose language is `mermaid` becomes a diagram:

````markdown
```mermaid
sequenceDiagram
    You->>webolator: docs/
    webolator->>Browser: index.html
```
````

```mermaid
sequenceDiagram
    You->>webolator: docs/
    webolator->>Browser: index.html
```

Diagrams follow the page's light or dark mode.

## Another example

```mermaid
stateDiagram-v2
    [*] --> Scan
    Scan --> Render
    Render --> Write
    Write --> [*]
    Write --> Scan: file changed (--serve)
```

## Offline use

By default mermaid.js is loaded from the jsDelivr CDN, and only on pages that contain a diagram. To avoid the network entirely, pass a local copy:

```bash
webolator docs/ --single --mermaid-js mermaid.min.js
```

In a static build the file is copied to `_webolator/`. With `--single` it is inlined into the output file.
