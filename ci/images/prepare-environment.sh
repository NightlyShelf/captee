#!/usr/bin/env bash
set -euo pipefail

role="${1:?usage: prepare-environment.sh test|build}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

case "$role" in
    test) image="${CAPTEE_TEST_IMAGE:-}"; healthcheck=/usr/local/bin/captee-test-healthcheck ;;
    build) image="${CAPTEE_BUILD_IMAGE:-}"; healthcheck=/usr/local/bin/captee-build-healthcheck ;;
    *) printf 'unknown image role: %s\n' "$role" >&2; exit 2 ;;
esac

write_env() {
    if [[ -n "${GITHUB_ENV:-}" ]]; then
        printf '%s\n' "$1" >> "$GITHUB_ENV"
    fi
}

if [[ -n "$image" ]]; then
    if [[ -n "${GHCR_TOKEN:-}" ]]; then
        printf '%s' "$GHCR_TOKEN" | docker login ghcr.io \
            --username "${GHCR_USERNAME:-github-actions[bot]}" --password-stdin >/dev/null
    fi
    if docker pull "$image" \
        && docker run --rm --pull=never "$image" "$healthcheck"; then
        write_env 'CAPTEE_ENV_MODE=image'
        write_env "CAPTEE_SELECTED_IMAGE=$image"
        printf 'Using immutable Captee %s image: %s\n' "$role" "$image"
        exit 0
    fi
    printf 'Captee %s image could not be pulled or failed readiness: %s\n' "$role" "$image" >&2
else
    printf 'CAPTEE_%s_IMAGE is not configured.\n' "$(tr '[:lower:]' '[:upper:]' <<< "$role")" >&2
fi

if [[ "${CAPTEE_ALLOW_RUNNER_FALLBACK:-false}" == true ]]; then
    printf 'Using documented Ubuntu runner fallback for %s.\n' "$role" >&2
    "$script_dir/runner-fallback-setup.sh" "$role"
    write_env 'CAPTEE_ENV_MODE=runner'
    write_env 'CAPTEE_SELECTED_IMAGE='
    exit 0
fi

cat >&2 <<EOF
Captee CI image setup failed for role '$role'.
Set the repository variable CAPTEE_${role^^}_IMAGE to a validated immutable
GHCR digest reference, or enable CAPTEE_ALLOW_RUNNER_FALLBACK for migration.
EOF
exit 1
