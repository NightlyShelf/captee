#!/usr/bin/env bash
set -euo pipefail

expected_rust="${CAPTEE_RUST_TOOLCHAIN:-1.97.1}"
manifest="${1:-/opt/captee-manifests/Cargo.toml}"

test "$(uname -m)" = "x86_64"
rustc --version | grep -F "${expected_rust}" >/dev/null
cargo --version | grep -F "${expected_rust}" >/dev/null
rustup toolchain list | grep -F "${expected_rust}" >/dev/null
rustup component list --toolchain "${expected_rust}" --installed | grep -E '^rustfmt|^clippy' >/dev/null
rustfmt --version >/dev/null
cargo clippy --version >/dev/null
test -d "${CARGO_TARGET_DIR:?}/release/deps"
command -v pkg-config >/dev/null
pkg-config --exists gtk4 gtksourceview-5
pkg-config --modversion gtk4
pkg-config --modversion gtksourceview-5
test -x "${LINUXDEPLOY_BIN:-/opt/captee-packaging/linuxdeploy}"
test -x "${LINUXDEPLOY_GTK_PLUGIN:-/opt/captee-packaging/linuxdeploy-plugin-gtk}"
test -x "${APPIMAGETOOL_BIN:-/opt/captee-packaging/appimagetool}"
test -f "${APPIMAGE_RUNTIME_FILE:-/opt/captee-packaging/runtime-x86_64}"
command -v mksquashfs >/dev/null
command -v patchelf >/dev/null

if [[ -f "$manifest" ]]; then
    cargo metadata --locked --no-deps --manifest-path "$manifest" >/dev/null
fi

APPIMAGE_EXTRACT_AND_RUN=1 "${LINUXDEPLOY_BIN:-/opt/captee-packaging/linuxdeploy}" --version >/dev/null
APPIMAGE_EXTRACT_AND_RUN=1 "${APPIMAGETOOL_BIN:-/opt/captee-packaging/appimagetool}" --version >/dev/null

printf 'Captee build image ready: arch=%s rust=%s gtk=%s gtksourceview=%s packaging=%s\n' \
    "$(uname -m)" \
    "$(rustc --version)" \
    "$(pkg-config --modversion gtk4)" \
    "$(pkg-config --modversion gtksourceview-5)" \
    "$(basename "${LINUXDEPLOY_BIN:-/opt/captee-packaging/linuxdeploy}")"
