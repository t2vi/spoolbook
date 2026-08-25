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
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY spoolbook-rs/ ./
RUN cargo build --release

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend-build /src/target/release/spoolbook-rs .
COPY --from=frontend-build /src/spoolbook-web-svelte/build ./svelte-build

ENV SPOOLBOOK_STATIC_ROOT=/app/svelte-build
ENV SPOOLBOOK_DB_PATH=/data/spoolbook-rs.db
VOLUME /data
EXPOSE 5070

ENTRYPOINT ["./spoolbook-rs"]
