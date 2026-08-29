# Build context is the repo root (docker-compose.yml sets it) since this needs both
# spoolbook-rs and spoolbook-web-svelte (the frontend) as siblings.

FROM node:22-slim AS frontend-build
WORKDIR /src/spoolbook-web-svelte
COPY spoolbook-web-svelte/package.json spoolbook-web-svelte/package-lock.json ./
RUN npm ci
COPY spoolbook-web-svelte/ ./
RUN npm run build

FROM rust:1-slim AS backend-build
WORKDIR /src
# libcurl4-openssl-dev (not curl's own static-ssl/vendored-OpenSSL feature -- see send_print.rs's
# upload_via_ftps doc comment): the printer's vsftpd requires TLS session reuse between its
# control and data connections, which a from-source-vendored OpenSSL build fails at (confirmed
# against real hardware), but Debian's own packaged libcurl+OpenSSL handles correctly.
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev libcurl4-openssl-dev && rm -rf /var/lib/apt/lists/*
COPY spoolbook-rs/ ./
RUN cargo build --release

FROM debian:trixie-slim
# ffmpeg: printer_camera.rs shells out to it for the live-camera MJPEG relay -- without it the
# camera feed fails with a generic "Camera stream failed" on every Docker install (works in dev
# only because macOS has ffmpeg on PATH). Pulls ~150MB of libav* deps; a static ffmpeg build
# would shrink that if image size ever matters.
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libcurl4t64 ffmpeg && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend-build /src/target/release/spoolbook-rs .
COPY --from=frontend-build /src/spoolbook-web-svelte/build ./svelte-build

ENV SPOOLBOOK_STATIC_ROOT=/app/svelte-build
ENV SPOOLBOOK_DB_PATH=/data/spoolbook-rs.db
VOLUME /data
EXPOSE 5070

ENTRYPOINT ["./spoolbook-rs"]
