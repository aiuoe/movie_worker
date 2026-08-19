# syntax=docker/dockerfile:1.7
# ──────────────────────────────────────────────────────────────────────────────
# movie_worker — Rust + axum background jobs
#
# Checkout del commit adentro (no COPY del build context).
# Args:
#   REPO   = aiuoe/movie_worker
#   COMMIT = branch / tag / sha  (default: main)
# ──────────────────────────────────────────────────────────────────────────────

ARG REPO=aiuoe/movie_worker
ARG COMMIT=main

FROM alpine/git:latest AS src
ARG REPO
ARG COMMIT
RUN apk add --no-cache bash && \
    git clone --filter=blob:none --no-checkout https://github.com/${REPO}.git /src && \
    cd /src && \
    if echo "${COMMIT}" | grep -qE '^[0-9a-f]{7,}$'; then \
      git fetch --depth 1 origin ${COMMIT} && git checkout ${COMMIT}; \
    else \
      git checkout ${COMMIT}; \
    fi

# ──────────────────────────────────────────────────────────────────────────────
# Stage 1: build con Rust + musl (binario estático)
# ──────────────────────────────────────────────────────────────────────────────
FROM rust:1.91-alpine AS build

WORKDIR /src
RUN apk add --no-cache musl-dev pkgconfig bash

# Cache de crates
COPY --from=src /src/Cargo.toml /src/Cargo.lock* ./
RUN mkdir -p src && echo "fn main(){}" > src/main.rs && \
    cargo build --release --target x86_64-unknown-linux-musl && \
    rm -rf src target/x86_64-unknown-linux-musl/release/deps/movie_worker*

COPY --from=src /src/ ./
RUN cargo build --release --target x86_64-unknown-linux-musl \
    && strip target/x86_64-unknown-linux-musl/release/movie_worker

# ──────────────────────────────────────────────────────────────────────────────
# Stage 2: distroless cc
# ──────────────────────────────────────────────────────────────────────────────
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

ARG COMMIT
ARG REPO
LABEL org.opencontainers.image.title="movie_worker" \
      org.opencontainers.image.source="https://github.com/${REPO}" \
      org.opencontainers.image.revision="${COMMIT}" \
      org.opencontainers.image.licenses="MIT"

COPY --from=build /src/target/x86_64-unknown-linux-musl/release/movie_worker /movie_worker

ENV ADDR=0.0.0.0:9090 \
    API_URL=http://movie_api:8080 \
    MEDIA_ROOT=/media

EXPOSE 9090

USER nonroot:nonroot

ENTRYPOINT ["/movie_worker"]