# ---- 1. Build the web UI ----
FROM node:20-slim AS web
WORKDIR /web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# ---- 2. Build the server (release) ----
FROM rust:1-slim-bookworm AS server
# mp3lame-encoder + bundled SQLite (rusqlite) need a C toolchain.
RUN apt-get update && apt-get install -y --no-install-recommends build-essential \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ crates/
COPY migrations/ migrations/
RUN cargo build --release -p aftercode-server

# ---- 3. Runtime ----
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=server /app/target/release/aftercode-server /usr/local/bin/aftercode-server
COPY --from=web /web/dist /app/web/dist
# Run as a non-root user; own the data dir so SQLite + audio are writable.
RUN useradd --system --uid 10001 --create-home app \
    && mkdir -p /data && chown -R app:app /data /app
ENV WEB_DIST=/app/web/dist \
    BIND_ADDR=0.0.0.0:8080 \
    DATABASE_URL=sqlite:///data/aftercode.db?mode=rwc \
    BLOB_STORE=localfs \
    LOCALFS_DIR=/data/audio \
    APP_PUBLIC_URL=http://localhost:8080
EXPOSE 8080
VOLUME ["/data"]
USER app
CMD ["aftercode-server"]
