# Dependency policy

The workspace currently has no third-party runtime dependencies. New crates
must use a specific version (no wildcard requirements), pass `cargo deny check`,
and have a license compatible with the allow-list in `deny.toml`.

`captee-core` must remain dependency-light and headless. GTK, Linux capture,
filesystem, process, and bundled Typst integrations belong behind the platform
or UI crate boundaries. Any dependency that crosses those boundaries requires a
design update and a CI fixture or test double.

