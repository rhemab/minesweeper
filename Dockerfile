# ---------- Planner ----------
FROM rust:1.93.1 AS planner
WORKDIR /app

RUN cargo install cargo-chef

COPY . .

RUN cargo chef prepare --recipe-path recipe.json


# ---------- Dependency builder ----------
FROM rust:1.76 AS builder
WORKDIR /app

RUN cargo install cargo-chef

COPY --from=planner /app/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json

COPY . .

RUN cargo build --release -p server


# ---------- Runtime ----------
FROM debian:bookworm-slim

WORKDIR /app

COPY --from=builder /app/target/release/server /usr/local/bin/server

CMD ["server"]
