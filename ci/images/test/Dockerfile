FROM ubuntu:22.04@sha256:0199853f6d6b20b0424f3c5694a72a62764f01e6a771b1eb48a4197848986c7e

ARG IMAGE_VERSION=2026.08.14.1
ARG IMAGE_SOURCE=https://github.com/NightlyShelf/captee

ENV CARGO_HOME=/usr/local/cargo \
    CARGO_INCREMENTAL=0 \
    CARGO_TARGET_DIR=/opt/captee-target \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:$PATH \
    DEBIAN_FRONTEND=noninteractive \
    CAPTEE_IMAGE_ROLE=test \
    CAPTEE_IMAGE_VERSION=${IMAGE_VERSION}

LABEL org.opencontainers.image.title="Captee CI test environment" \
      org.opencontainers.image.description="Pinned Rust, GTK, and GtkSourceView test environment for Captee" \
      org.opencontainers.image.source="${IMAGE_SOURCE}" \
      org.opencontainers.image.version="${IMAGE_VERSION}" \
      org.opencontainers.image.architecture="amd64" \
      org.opencontainers.image.base.name="ubuntu:22.04@sha256:0199853f6d6b20b0424f3c5694a72a62764f01e6a771b1eb48a4197848986c7e"

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

COPY ci/images/image-version.env /tmp/captee-image-version.env
COPY ci/images/test-packages.txt /tmp/captee-test-packages.txt

RUN set -eux; \
    source /tmp/captee-image-version.env; \
    mapfile -t packages < <(grep -Ev '^[[:space:]]*(#|$)' /tmp/captee-test-packages.txt); \
    apt-get update; \
    apt-get install --yes --no-install-recommends \
        "${packages[@]}" \
        "libgtk-4-dev=${GTK_DEV_VERSION}" \
        "libgtksourceview-5-dev=${GTK_SOURCE_DEV_VERSION}"; \
    rm -rf /var/lib/apt/lists/* /tmp/captee-test-packages.txt

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

RUN cargo clippy --locked --workspace --all-targets --all-features \
    && cargo test --locked --workspace --no-run

COPY ci/images/test/healthcheck.sh /usr/local/bin/captee-test-healthcheck
RUN chmod 0755 /usr/local/bin/captee-test-healthcheck

HEALTHCHECK --interval=30s --timeout=15s --retries=1 CMD ["/usr/local/bin/captee-test-healthcheck"]

WORKDIR /workspace
CMD ["bash"]
