# Single-file output

```bash
webolator docs/ --single -o docs.html
```

This produces one `.html` file that you can email, attach to a ticket or drop anywhere. [This documentation as a single file](https://vivainio.github.io/webolator/webolator-docs.html) is an example.

## What's inside

- Every page becomes a `<section>`, and only one is shown at a time.
- Links become `#page-id` or `#page-id--heading`, so the back and forward buttons and deep links work.
- Images are inlined.
- Raw `.html` files are shown in a frame, with a link to open them on their own.
- Text files (`.txt`, `.log`, source code) are shown as preformatted text.
- PDFs and other files are embedded and open in a new tab when clicked.
- Mermaid diagrams are drawn when their section is first shown.

## Size

Everything is embedded, so large PDFs or images make the file large. mermaid.js is loaded from the CDN unless you pass `--mermaid-js`.
