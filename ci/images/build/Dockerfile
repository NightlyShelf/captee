FROM ubuntu:22.04@sha256:0199853f6d6b20b0424f3c5694a72a62764f01e6a771b1eb48a4197848986c7e

ARG IMAGE_VERSION=2026.08.14.1
ARG IMAGE_SOURCE=https://github.com/NightlyShelf/captee

ENV CARGO_HOME=/usr/local/cargo \
    CARGO_INCREMENTAL=0 \
    CARGO_TARGET_DIR=/opt/captee-target \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:/opt/captee-packaging:$PATH \
    DEBIAN_FRONTEND=noninteractive \
    APPIMAGE_EXTRACT_AND_RUN=1 \
    CAPTEE_IMAGE_ROLE=build \
    CAPTEE_IMAGE_VERSION=${IMAGE_VERSION} \
    LINUXDEPLOY_BIN=/opt/captee-packaging/linuxdeploy \
    LINUXDEPLOY_GTK_PLUGIN=/opt/captee-packaging/linuxdeploy-plugin-gtk \
    APPIMAGETOOL_BIN=/opt/captee-packaging/appimagetool \
    APPIMAGE_RUNTIME_FILE=/opt/captee-packaging/runtime-x86_64

LABEL org.opencontainers.image.title="Captee CI build environment" \
      org.opencontainers.image.description="Pinned Rust, GTK, and AppImage packaging environment for Captee" \
      org.opencontainers.image.source="${IMAGE_SOURCE}" \
      org.opencontainers.image.version="${IMAGE_VERSION}" \
      org.opencontainers.image.architecture="amd64" \
      org.opencontainers.image.base.name="ubuntu:22.04@sha256:0199853f6d6b20b0424f3c5694a72a62764f01e6a771b1eb48a4197848986c7e"

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

COPY ci/images/image-version.env /tmp/captee-image-version.env
COPY ci/images/build-packages.txt /tmp/captee-build-packages.txt

RUN set -eux; \
    source /tmp/captee-image-version.env; \
    mapfile -t packages < <(grep -Ev '^[[:space:]]*(#|$)' /tmp/captee-build-packages.txt); \
    apt-get update; \
    apt-get install --yes --no-install-recommends \
        "${packages[@]}" \
        "libgtk-4-dev=${GTK_DEV_VERSION}" \
        "libgtksourceview-5-dev=${GTK_SOURCE_DEV_VERSION}" \
        "fuse=${FUSE_VERSION}" \
        "libfuse2=${LIBFUSE2_VERSION}" \
        "squashfs-tools=${SQUASHFS_TOOLS_VERSION}" \
        "patchelf=${PATCHELF_VERSION}"; \
    rm -rf /var/lib/apt/lists/* /tmp/captee-build-packages.txt

RUN set -eux; \
    source /tmp/captee-image-version.env; \
    curl --fail --location --retry 3 --proto '=https' --tlsv1.2 https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain "${RUST_TOOLCHAIN}" \
          --component rustfmt --component clippy; \
    rustup set profile minimal; \
    rustup default "${RUST_TOOLCHAIN}"; \
    rustc --version; \
    cargo --version

WORKDIR /opt/captee-manifests
COPY Cargo.toml Cargo.lock rust-toolchain.toml rustfmt.toml ./
COPY crates/captee-core/Cargo.toml crates/captee-core/Cargo.toml
COPY crates/captee-platform/Cargo.toml crates/captee-platform/Cargo.toml
COPY crates/captee-ui/Cargo.toml crates/captee-ui/Cargo.toml

RUN mkdir -p crates/captee-core/src crates/captee-platform/src crates/captee-ui/src \
    && touch crates/captee-core/src/lib.rs crates/captee-platform/src/lib.rs \
        crates/captee-ui/src/lib.rs \
    && printf 'fn main() {}\n' > crates/captee-ui/src/main.rs

RUN cargo build --locked --release --package captee-ui

RUN set -eux; \
    source /tmp/captee-image-version.env; \
    mkdir -p /opt/captee-packaging; \
    curl --fail --location --retry 3 --proto '=https' --tlsv1.2 \
        --output /opt/captee-packaging/linuxdeploy "${LINUXDEPLOY_URL}"; \
    curl --fail --location --retry 3 --proto '=https' --tlsv1.2 \
        --output /opt/captee-packaging/linuxdeploy-plugin-gtk "${LINUXDEPLOY_GTK_PLUGIN_URL}"; \
    curl --fail --location --retry 3 --proto '=https' --tlsv1.2 \
        --output /opt/captee-packaging/appimagetool "${APPIMAGETOOL_URL}"; \
    curl --fail --location --retry 3 --proto '=https' --tlsv1.2 \
        --output /opt/captee-packaging/runtime-x86_64 "${APPIMAGE_RUNTIME_URL}"; \
    printf '%s  %s\n' "${LINUXDEPLOY_SHA256}" /opt/captee-packaging/linuxdeploy | sha256sum -c -; \
    printf '%s  %s\n' "${LINUXDEPLOY_GTK_PLUGIN_SHA256}" /opt/captee-packaging/linuxdeploy-plugin-gtk | sha256sum -c -; \
    printf '%s  %s\n' "${APPIMAGETOOL_SHA256}" /opt/captee-packaging/appimagetool | sha256sum -c -; \
    printf '%s  %s\n' "${APPIMAGE_RUNTIME_SHA256}" /opt/captee-packaging/runtime-x86_64 | sha256sum -c -; \
    chmod 0755 /opt/captee-packaging/*; \
    rm -f /tmp/captee-image-version.env

COPY ci/images/build/healthcheck.sh /usr/local/bin/captee-build-healthcheck
RUN chmod 0755 /usr/local/bin/captee-build-healthcheck

HEALTHCHECK --interval=30s --timeout=30s --retries=1 CMD ["/usr/local/bin/captee-build-healthcheck"]

WORKDIR /workspace
CMD ["bash"]
