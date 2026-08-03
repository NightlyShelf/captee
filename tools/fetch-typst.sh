#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root_dir/tools/typst-version.toml"
output_dir="${1:-$root_dir/dist/typst}"
archive_name="typst-x86_64-unknown-linux-musl.tar.xz"
archive_url="https://github.com/typst/typst/releases/download/v0.14.2/$archive_name"
archive_sha256="a6044cbad2a954deb921167e257e120ac0a16b20339ec01121194ff9d394996d"
archive_path="$output_dir/$archive_name"

test -f "$manifest"
mkdir -p "$output_dir"
curl --fail --location --retry 3 --output "$archive_path" "$archive_url"
printf '%s  %s\n' "$archive_sha256" "$archive_path" | sha256sum --check --status
tar --extract --file "$archive_path" --xz --strip-components=1 --directory "$output_dir"
rm -f "$archive_path"
test -x "$output_dir/typst"
printf 'Typst %s installed at %s\n' "$($output_dir/typst --version)" "$output_dir/typst"

