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
test -d "${CARGO_TARGET_DIR:?}/debug/deps"
command -v gcc >/dev/null
command -v pkg-config >/dev/null
pkg-config --exists gtk4 gtksourceview-5
pkg-config --modversion gtk4
pkg-config --modversion gtksourceview-5

if [[ -f "$manifest" ]]; then
    cargo metadata --locked --no-deps --manifest-path "$manifest" >/dev/null
fi

printf 'Captee test image ready: arch=%s rust=%s gtk=%s gtksourceview=%s\n' \
    "$(uname -m)" \
    "$(rustc --version)" \
    "$(pkg-config --modversion gtk4)" \
    "$(pkg-config --modversion gtksourceview-5)"
