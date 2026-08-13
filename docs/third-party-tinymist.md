# Bundled Tinymist

Captee bundles Tinymist 0.14.6 for x86_64 Linux editor assistance. This
version embeds Typst 0.14.2, matching Captee's bundled compiler. The selected
upstream asset is `tinymist-x86_64-unknown-linux-musl.tar.gz` from the
[official Tinymist release](https://github.com/Myriad-Dreamin/tinymist/releases/tag/v0.14.6).

The archive size is 30,289,874 bytes and its SHA-256 digest is:

```text
1411a883e5409ea77cae50788d4149229ec6e8046dfb4b79072de72b49dbc720
```

`tools/fetch-tinymist.sh` verifies the archive and the tagged upstream
`LICENSE` before installation. Tinymist is licensed under Apache-2.0 and has
no upstream `NOTICE` file at this release. Its license remains beside the
binary in every AppImage.
