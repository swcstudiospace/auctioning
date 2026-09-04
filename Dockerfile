# auctioning-api-runner — the Shuttle-free Axum binary.
# Shuttle remains the primary deploy; this image is for Railway/Fly/self-host.

FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY programs ./programs
COPY backend ./backend
COPY magicblock ./magicblock
COPY app ./app
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release -p shuttle-auctioning --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY programs ./programs
COPY backend ./backend
COPY magicblock ./magicblock
COPY app ./app
RUN cargo build --release -p shuttle-auctioning --bin auctioning-api-runner

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --create-home app
WORKDIR /app
COPY --from=builder /app/target/release/auctioning-api-runner /usr/local/bin/auctioning-api-runner
USER app
ENV BIND_ADDR=0.0.0.0:8000 RUST_LOG=info,sqlx=warn
EXPOSE 8000
HEALTHCHECK --interval=30s --timeout=3s --start-period=20s \
  CMD curl -fsS http://127.0.0.1:8000/healthz || exit 1
ENTRYPOINT ["/usr/local/bin/auctioning-api-runner"]
