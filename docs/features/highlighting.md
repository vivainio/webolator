# Syntax highlighting

Code blocks are highlighted at build time with [syntect](https://github.com/trishume/syntect), so no JavaScript is involved and highlighting also works in single-file output. Colors follow the reader's light or dark mode.

```rust
fn main() {
    let pages = ["index.md", "usage.md"];
    for p in pages.iter().filter(|p| p.ends_with(".md")) {
        println!("rendering {p}");
    }
}
```

```python
from pathlib import Path

def titles(root: Path) -> dict[str, str]:
    """First heading of every markdown file."""
    return {p.name: p.read_text().splitlines()[0].lstrip("# ") for p in root.rglob("*.md")}
```

```bash
for f in docs/*.md; do
  echo "$f: $(head -1 "$f")"
done
```

```json
{ "name": "webolator", "config": null }
```

Use the language's name or file extension after the fence (`rust`, `py`, `sh` and so on). A block with no language, or a language syntect doesn't know, is shown as plain text.
