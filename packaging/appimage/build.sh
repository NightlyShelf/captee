#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
build_root="${CAPTEE_APPIMAGE_BUILD_DIR:-$project_root/dist/appimage}"
if [[ "$build_root" != /* ]]; then
    build_root="$project_root/$build_root"
fi
appdir="$build_root/AppDir"
linuxdeploy_bin="${LINUXDEPLOY_BIN:-$(command -v linuxdeploy || true)}"
gtk_plugin="${LINUXDEPLOY_GTK_PLUGIN:-$(command -v linuxdeploy-plugin-gtk || true)}"
appimagetool_bin="${APPIMAGETOOL_BIN:-$(command -v appimagetool || true)}"
runtime_file="${APPIMAGE_RUNTIME_FILE:-}"
desktop_file="$project_root/packaging/appimage/com.nightlyshelf.Captee.desktop"
icon_file="$project_root/packaging/appimage/captee.svg"

if [[ -z "$linuxdeploy_bin" || ! -x "$linuxdeploy_bin" ]]; then
    printf '%s\n' 'linuxdeploy is required; set LINUXDEPLOY_BIN or add it to PATH' >&2
    exit 1
fi
if [[ -z "$gtk_plugin" || ! -x "$gtk_plugin" ]]; then
    printf '%s\n' 'linuxdeploy-plugin-gtk is required; set LINUXDEPLOY_GTK_PLUGIN or add it to PATH' >&2
    exit 1
fi
if [[ -n "$appimagetool_bin" && -z "$runtime_file" ]]; then
    printf '%s\n' 'APPIMAGE_RUNTIME_FILE is required when using an external appimagetool' >&2
    exit 1
fi
if [[ -n "$runtime_file" && ! -f "$runtime_file" ]]; then
    printf 'runtime file does not exist: %s\n' "$runtime_file" >&2
    exit 1
fi

mkdir -p "$appdir/usr/bin" "$appdir/usr/share/applications"
cargo build --release --manifest-path "$project_root/Cargo.toml" -p captee-ui
cp "$project_root/target/release/captee-ui" "$appdir/usr/bin/captee-ui"
cp "$desktop_file" "$appdir/usr/share/applications/com.nightlyshelf.Captee.desktop"
"$project_root/tools/fetch-typst.sh" "$appdir/usr/share/captee/typst"

export PATH="$(dirname "$gtk_plugin"):$PATH"
cd "$build_root"
linuxdeploy_args=( \
    --appdir "$appdir" \
    --executable "$appdir/usr/bin/captee-ui" \
    --desktop-file "$appdir/usr/share/applications/com.nightlyshelf.Captee.desktop" \
    --icon-file "$icon_file" \
    --icon-filename captee \
    --plugin gtk
)

if [[ -n "$appimagetool_bin" && -x "$appimagetool_bin" ]]; then
    "$linuxdeploy_bin" "${linuxdeploy_args[@]}"
    appimagetool_args=()
    if [[ -n "$runtime_file" ]]; then
        appimagetool_args+=(--runtime-file "$runtime_file")
    fi
    "$appimagetool_bin" "${appimagetool_args[@]}" "$appdir"
else
    "$linuxdeploy_bin" "${linuxdeploy_args[@]}" --output appimage
fi
