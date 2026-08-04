#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
manifest="${CAPTEE_IMAGE_VERSION_FILE:-$script_dir/image-version.env}"
destination="${1:?usage: fetch-packaging-tools.sh DESTINATION}"

source "$manifest"
mkdir -p "$destination"

download_and_verify() {
    local url="$1"
    local sha256="$2"
    local output="$3"

    curl --fail --location --retry 3 --proto '=https' --tlsv1.2 \
        --output "$output" "$url"
    printf '%s  %s\n' "$sha256" "$output" | sha256sum -c -
}

download_and_verify "$LINUXDEPLOY_URL" "$LINUXDEPLOY_SHA256" "$destination/linuxdeploy"
download_and_verify "$LINUXDEPLOY_GTK_PLUGIN_URL" "$LINUXDEPLOY_GTK_PLUGIN_SHA256" \
    "$destination/linuxdeploy-plugin-gtk"
download_and_verify "$APPIMAGETOOL_URL" "$APPIMAGETOOL_SHA256" "$destination/appimagetool"
download_and_verify "$APPIMAGE_RUNTIME_URL" "$APPIMAGE_RUNTIME_SHA256" \
    "$destination/runtime-x86_64"
chmod 0755 "$destination"/*

sha256sum "$destination"/*
