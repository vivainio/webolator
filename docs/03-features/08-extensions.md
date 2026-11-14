# Tabs, admonitions, snippets and galleries

The Python-Markdown / MkDocs syntax for these is supported, so existing content works unchanged. Bodies are indented four spaces. Fenced code blocks are never touched.

## Content tabs

````markdown
=== "Linux"

    ```bash
    apt install webolator
    ```

=== "macOS"

    ```bash
    brew install webolator
    ```
````

Consecutive tabs form one group. Tabs can contain any markdown, including diagrams and nested tabs.

## Admonitions

```markdown
!!! warning "Careful"

    This overwrites the output folder.

??? tip "Click to open"
    Collapsed by default.

???+ note "Open by default"
    Use `???+` to start expanded.
```

The title is optional (`!!! note` uses "Note"). Types `note`, `tip`, `important`, `warning` and `caution` have their own colours, and others such as `info`, `success`, `question`, `danger`, `bug` and `example` map onto them. GitHub-style `> [!NOTE]` alerts keep working.

## Snippets

`--8<-- "path/to/file"` on its own line is replaced by that file's contents. The path is tried relative to the page, then to the docs folder.

## Code block titles

````markdown
```python title="app.py"
print("hello")
```
````

## Galleries

A `gallery` fence puts a thumbnail grid anywhere on a page. List a folder, image files or a glob, one per line, relative to the page (or to the site root with a leading `/`):

````markdown
```gallery
screenshots/
```

```gallery
img/login.png
img/dash*.png
```
````

Lines starting with `#` are ignored, and images shown twice are only listed once. Thumbnails are made when the site is built, a click opens the full image, and the arrow keys step through the images in that gallery. In `--single` output the images are embedded. Unlike `--gallery`, the images stay in the rest of the site, and the page around the gallery is normal markdown.

Use `--gallery DIR` instead when you want a whole folder to become a page of its own.
