#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root_dir/tools/tinymist-version.env"
output_dir="${1:-$root_dir/dist/tinymist}"

test -f "$manifest"
source "$manifest"
: "${TINYMIST_ARCHIVE:?}"
: "${TINYMIST_URL:?}"
: "${TINYMIST_SHA256:?}"
: "${TINYMIST_LICENSE_URL:?}"
: "${TINYMIST_LICENSE_SHA256:?}"

archive_path="$output_dir/$TINYMIST_ARCHIVE"
license_path="$output_dir/LICENSE"
mkdir -p "$output_dir"
curl --fail --location --retry 3 --output "$archive_path" "$TINYMIST_URL"
printf '%s  %s\n' "$TINYMIST_SHA256" "$archive_path" | sha256sum --check --status
tar --extract --file "$archive_path" --gzip --strip-components=1 --directory "$output_dir"
rm -f "$archive_path"
curl --fail --location --retry 3 --output "$license_path" "$TINYMIST_LICENSE_URL"
printf '%s  %s\n' "$TINYMIST_LICENSE_SHA256" "$license_path" | sha256sum --check --status
test -x "$output_dir/tinymist"
printf 'Tinymist %s installed at %s\n' "$($output_dir/tinymist --version)" "$output_dir/tinymist"
