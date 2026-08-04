#!/usr/bin/env bash
set -euo pipefail

if [[ "${CAPTEE_ENV_MODE:-runner}" != image ]]; then
    exec "$@"
fi

image="${CAPTEE_SELECTED_IMAGE:?CAPTEE_SELECTED_IMAGE is required in image mode}"
workspace="${GITHUB_WORKSPACE:-$PWD}"
docker_args=(
    run --rm --init --pull=never
    --volume "$workspace:/workspace"
    --workdir /workspace
)

for variable in \
    APPIMAGE_EXTRACT_AND_RUN \
    CAPTEE_APPIMAGE_BUILD_DIR \
    CARGO_TERM_COLOR \
    LINUXDEPLOY_BIN \
    LINUXDEPLOY_GTK_PLUGIN \
    APPIMAGETOOL_BIN \
    APPIMAGE_RUNTIME_FILE; do
    if [[ -n "${!variable+x}" ]]; then
        docker_args+=(--env "$variable=${!variable}")
    fi
done

exec docker "${docker_args[@]}" "$image" "$@"
