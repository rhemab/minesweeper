# ---------- Planner ----------
FROM rust:1.93.1 AS planner
WORKDIR /app

# ---------- Dependency builder ----------
FROM rust:1.76 AS builder
WORKDIR /app

COPY . .

RUN cargo build --release -p server


# ---------- Runtime ----------
FROM debian:bookworm-slim

WORKDIR /app

COPY --from=builder /app/target/release/server /usr/local/bin/server

CMD ["server"]
