#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root_dir/tools/tinymist-version.toml"
output_dir="${1:-$root_dir/dist/tinymist}"
archive_name="tinymist-x86_64-unknown-linux-musl.tar.gz"
archive_url="https://github.com/Myriad-Dreamin/tinymist/releases/download/v0.14.6/$archive_name"
archive_sha256="1411a883e5409ea77cae50788d4149229ec6e8046dfb4b79072de72b49dbc720"
license_url="https://raw.githubusercontent.com/Myriad-Dreamin/tinymist/v0.14.6/LICENSE"
license_sha256="a9f29769fd3a7ee2976e6e161a93e16461fa305c088c4806242e50ec8ef86bce"
archive_path="$output_dir/$archive_name"
license_path="$output_dir/LICENSE"

test -f "$manifest"
mkdir -p "$output_dir"
curl --fail --location --retry 3 --output "$archive_path" "$archive_url"
printf '%s  %s\n' "$archive_sha256" "$archive_path" | sha256sum --check --status
tar --extract --file "$archive_path" --gzip --strip-components=1 --directory "$output_dir"
rm -f "$archive_path"
curl --fail --location --retry 3 --output "$license_path" "$license_url"
printf '%s  %s\n' "$license_sha256" "$license_path" | sha256sum --check --status
test -x "$output_dir/tinymist"
printf 'Tinymist %s installed at %s\n' "$($output_dir/tinymist --version)" "$output_dir/tinymist"
