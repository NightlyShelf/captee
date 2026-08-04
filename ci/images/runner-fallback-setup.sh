#!/usr/bin/env bash
set -euo pipefail

role="${1:?usage: runner-fallback-setup.sh test|build}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/image-version.env"

case "$role" in
    test)
        package_file="$script_dir/test-packages.txt"
        ;;
    build)
        package_file="$script_dir/build-packages.txt"
        ;;
    *)
        printf 'unknown fallback role: %s\n' "$role" >&2
        exit 2
        ;;
esac

mapfile -t packages < <(grep -Ev '^[[:space:]]*(#|$)' "$package_file")
sudo apt-get update
sudo DEBIAN_FRONTEND=noninteractive apt-get install --yes --no-install-recommends \
    "${packages[@]}" \
    "libgtk-4-dev=${GTK_DEV_VERSION}" \
    "libgtksourceview-5-dev=${GTK_SOURCE_DEV_VERSION}"

if [[ "$role" == build ]]; then
    sudo DEBIAN_FRONTEND=noninteractive apt-get install --yes --no-install-recommends \
        "fuse=${FUSE_VERSION}" \
        "libfuse2=${LIBFUSE2_VERSION}" \
        "squashfs-tools=${SQUASHFS_TOOLS_VERSION}" \
        "patchelf=${PATCHELF_VERSION}"
fi

if ! command -v rustup >/dev/null; then
    curl --fail --location --retry 3 --proto '=https' --tlsv1.2 https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain none
    export PATH="$HOME/.cargo/bin:$PATH"
fi
rustup toolchain install "$RUST_TOOLCHAIN" --profile minimal \
    --component rustfmt --component clippy --no-self-update
rustup default "$RUST_TOOLCHAIN"

if [[ "$role" == build ]]; then
    packaging_dir="${CAPTEE_PACKAGING_DIR:-${RUNNER_TEMP:-$PWD/.ci}/captee-packaging}"
    CAPTEE_IMAGE_VERSION_FILE="$script_dir/image-version.env" \
        "$script_dir/fetch-packaging-tools.sh" "$packaging_dir"
    export LINUXDEPLOY_BIN="$packaging_dir/linuxdeploy"
    export LINUXDEPLOY_GTK_PLUGIN="$packaging_dir/linuxdeploy-plugin-gtk"
    export APPIMAGETOOL_BIN="$packaging_dir/appimagetool"
    export APPIMAGE_RUNTIME_FILE="$packaging_dir/runtime-x86_64"
    export APPIMAGE_EXTRACT_AND_RUN=1
    if [[ -n "${GITHUB_ENV:-}" ]]; then
        {
            printf 'LINUXDEPLOY_BIN=%s\n' "$packaging_dir/linuxdeploy"
            printf 'LINUXDEPLOY_GTK_PLUGIN=%s\n' "$packaging_dir/linuxdeploy-plugin-gtk"
            printf 'APPIMAGETOOL_BIN=%s\n' "$packaging_dir/appimagetool"
            printf 'APPIMAGE_RUNTIME_FILE=%s\n' "$packaging_dir/runtime-x86_64"
            printf 'APPIMAGE_EXTRACT_AND_RUN=1\n'
        } >> "$GITHUB_ENV"
    fi
fi

"$script_dir/${role}/healthcheck.sh"
