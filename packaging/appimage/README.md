# AppImage packaging

The release target is an x86_64 AppImage built on Ubuntu 22.04. The build
must use the pinned Rust toolchain, bundled Typst 0.14.2, and a GTK runtime
collected by `linuxdeploy-plugin-gtk`.

Required tools:

- Rust 1.97.1
- `libgtk-4-dev` and `libgtksourceview-5-dev`
- `linuxdeploy` and `linuxdeploy-plugin-gtk`
- `appimagetool` and the matching Type-2 `runtime-x86_64`

Run `packaging/appimage/build.sh`. The script requires the packaging tools to
be supplied explicitly through `LINUXDEPLOY_BIN`, `LINUXDEPLOY_GTK_PLUGIN`,
`APPIMAGETOOL_BIN`, and `APPIMAGE_RUNTIME_FILE`, or discover them on `PATH`
when the embedded tool can provide a runtime. This keeps downloads and trust
decisions outside the build script. The output is written below
`dist/appimage/` and must be tested on a clean Ubuntu 22.04 VM before release.

The script also fetches and verifies the pinned Typst and Tinymist archives
into the AppDir using `tools/fetch-typst.sh` and `tools/fetch-tinymist.sh`.
Record the exact linuxdeploy, plugin,
appimagetool, and runtime versions and SHA-256 digests with each artifact.

The desktop entry is `packaging/appimage/com.nightlyshelf.captee.desktop`.
Record the exact linuxdeploy/appimagetool versions and SHA-256 digests with
each release artifact.
