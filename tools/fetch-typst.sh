#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root_dir/tools/typst-version.env"
output_dir="${1:-$root_dir/dist/typst}"

test -f "$manifest"
source "$manifest"
: "${TYPST_ARCHIVE:?}"
: "${TYPST_URL:?}"
: "${TYPST_SHA256:?}"
: "${TYPST_SIZE:?}"

archive_path="$output_dir/$TYPST_ARCHIVE"
mkdir -p "$output_dir"
curl --fail --location --retry 3 --output "$archive_path" "$TYPST_URL"
test "$(stat --format=%s "$archive_path")" -eq "$TYPST_SIZE"
printf '%s  %s\n' "$TYPST_SHA256" "$archive_path" | sha256sum --check --status
tar --extract --file "$archive_path" --xz --strip-components=1 --directory "$output_dir"
rm -f "$archive_path"
test -x "$output_dir/typst"
printf 'Typst %s installed at %s\n' "$($output_dir/typst --version)" "$output_dir/typst"
