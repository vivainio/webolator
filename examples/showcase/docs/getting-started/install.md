# Install

Pick your platform. Tabs are plain markdown, so each one can hold anything.

=== "uv"

    ```bash
    uv tool install webolator
    ```

    Or try it without installing:

    ```bash
    uvx webolator docs/ --serve
    ```

=== "Cargo"

    ```bash
    cargo install --git https://github.com/vivainio/webolator
    ```

=== "Binary"

    Download the archive for your platform from the releases page and put `webolator` on your `PATH`.

    !!! note "Apple Silicon"

        There is a separate `macos-arm64` build.

Then continue with [your first page](first-page.md).
