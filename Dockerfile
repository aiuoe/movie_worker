# syntax=docker/dockerfile:1.7
# ──────────────────────────────────────────────────────────────────────────────
# Stage 1: build con Rust + musl (binario estático, sin glibc)
# ──────────────────────────────────────────────────────────────────────────────
FROM rust:1.91-alpine AS build

WORKDIR /src

# musl-dev para que reqwest/hyper-rustls compilen estáticos
RUN apk add --no-cache musl-dev pkgconfig

# Cache de crates: copiar solo manifests primero
COPY Cargo.toml Cargo.lock* ./
RUN mkdir -p src && echo "fn main(){}" > src/main.rs && \
    cargo build --release --target x86_64-unknown-linux-musl && \
    rm -rf src target/x86_64-unknown-linux-musl/release/deps/movie_worker*

COPY src ./src

RUN cargo build --release --target x86_64-unknown-linux-musl \
    && strip target/x86_64-unknown-linux-musl/release/movie_worker

# ──────────────────────────────────────────────────────────────────────────────
# Stage 2: distroless cc (C runtime mínimo, ffmpeg NO viene incluido).
# Si querés transcoding, usá el override en docker-compose.worker-ffmpeg.yml
# que monta `linuxserver/ffmpeg` o usa la variante con `ffmpeg` en $PATH.
# ──────────────────────────────────────────────────────────────────────────────
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

LABEL org.opencontainers.image.title="movie_worker" \
      org.opencontainers.image.source="https://example.local/movie_worker"

COPY --from=build /src/target/x86_64-unknown-linux-musl/release/movie_worker /movie_worker

ENV ADDR=0.0.0.0:9090 \
    API_URL=http://movie_api:8080 \
    MEDIA_ROOT=/media

EXPOSE 9090

USER nonroot:nonroot

ENTRYPOINT ["/movie_worker"]