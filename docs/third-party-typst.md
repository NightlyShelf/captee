# Bundled Typst

Captee bundles Typst 0.14.2 for the first x86_64 Linux AppImage. The selected
upstream asset is `typst-x86_64-unknown-linux-musl.tar.xz` and is fetched only
through [the official Typst release](https://github.com/typst/typst/releases/tag/v0.14.2).

The pinned archive size is 15,877,252 bytes and its SHA-256 digest is:

```text
a6044cbad2a954deb921167e257e120ac0a16b20339ec01121194ff9d394996d
```

`tools/fetch-typst.sh` verifies this digest before extracting the compiler and
retains the upstream `LICENSE` and `NOTICE` files in the distribution bundle.
Typst is licensed under Apache-2.0; its upstream notices remain part of every
release artifact. Additional targets require a separate manifest entry and
checksum before they are packaged.

