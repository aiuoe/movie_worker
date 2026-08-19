# movie_worker

Background jobs para el stack de streaming. Recibe órdenes de `movie_api`
y ejecuta trabajo CPU/IO bound: probe con ffprobe, transcode HLS, scrape de
metadata e ingest desde filesystem.

## Stack

- **Rust 1.91** (edition 2021)
- **axum 0.7** sobre **tokio** (full)
- **reqwest** con rustls (sin OpenSSL)
- **tracing** estructurado

## Endpoints internos

| Método | Ruta                | Descripción                              |
|--------|---------------------|------------------------------------------|
| GET    | `/healthz`          | Liveness                                 |
| POST   | `/jobs`             | Encola un job (`probe`/`transcode`/`scrape`/`ingest`) |
| GET    | `/jobs/:id`         | Estado del job                           |
| POST   | `/streams/notify`   | Notificación de nuevo stream desde movie_api |

## Comandos

```bash
cargo run --release        # http://localhost:9090
cargo build --release      # binario en target/release/movie_worker
```

## Variables de entorno

| Variable        | Default                       |
|-----------------|-------------------------------|
| `ADDR`          | `0.0.0.0:9090`                |
| `API_URL`       | `http://localhost:8080`       |
| `MEDIA_ROOT`    | `/media` (cuando se monte el storage) |
| `RUST_LOG`      | `info,movie_worker=debug`     |

## Estructura

```
movie_worker/src/
├── main.rs                # entrypoint
├── lib.rs                 # re-exports para tests/binarios auxiliares
├── config.rs              # env config
├── state.rs               # AppState (registry de jobs en memoria)
├── error.rs               # error type
├── server.rs              # router + middleware
├── client.rs              # HTTP client a movie_api (callbacks)
├── handlers/
│   ├── mod.rs
│   ├── health.rs
│   ├── jobs.rs
│   └── streams.rs
└── jobs/
    ├── mod.rs             # trait Job + dispatcher
    ├── probe.rs           # ffprobe de un archivo
    ├── transcode.rs       # ffmpeg → HLS (placeholder shell-out)
    ├── scrape.rs          # metadata desde TMDB (placeholder)
    └── ingest.rs          # scan de MEDIA_ROOT
```

## Hand-off con `movie_api`

El API reenvía `POST /api/jobs` → `POST /jobs` aquí. Cuando un job termina,
el worker hace `PATCH /api/internal/jobs/:id` en el API (callback) — el
endpoint se agrega en movie_api cuando se quiera estado distribuido.