# Search

Every site has a search box at the top of the sidebar. Press `/` to jump to it, use the arrow keys and Enter to pick a result, and Esc to close it. Results are sections of pages, so a search for "gallery" can take you straight to the heading that covers it.

## How it matches

- **Whole words, by prefix.** "install" finds "installation".
- **Every word counts.** A query of several words only matches sections that contain all of them.
- **Longer queries lose their ending.** A query is also tried without a common ending (`-ing`, `-ed`, `-s`, `-ation`), so "installation" finds a page that only says "install".
- **Headings rank higher** than body text.
- There is **no typo tolerance**, and no stemming beyond the above.

## How small it stays

The index is built when the site is, and is small on purpose. For each section of a page (split at headings) it stores the heading and the list of distinct lowercase words, with no sentences. Code blocks and diagrams are left out, and so are very common words. For this documentation that is about 16 KB, or 6 KB compressed.

It is **loaded on first use**. Nothing is fetched until someone clicks into the search box, so pages load exactly as fast as without search. The static site keeps the index in `_webolator/search.js`, which also works when you open the files straight from disk. In `--single` output the index is embedded in the file.

Pages with `hidden: true` are not searchable.

## Turning it off

```toml
[project]
search = false
```

This removes the search box, the index file and the keyboard shortcut.
