FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm

RUN apt-get update \
    && export DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y --no-install-recommends \
        jq \
        rsync \
        unzip \
        zip \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add aarch64-unknown-linux-musl
