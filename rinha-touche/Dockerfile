FROM rust:1-slim-buster AS build

RUN cargo new --bin app
WORKDIR /app
RUN cargo new --lib rinha-core
RUN cargo new --bin rinha-axum
RUN cargo new --bin rinha-touche
RUN mv /app/src/main.rs /app/rinha-touche/src/main.rs

COPY Cargo.toml /app/
COPY Cargo.lock /app/
COPY .cargo /app/.cargo
COPY rinha-core/Cargo.toml /app/rinha-core/
COPY rinha-axum/Cargo.toml /app/rinha-axum/
COPY rinha-touche/Cargo.toml /app/rinha-touche/
RUN cargo build --release -p rinha-touche

COPY rinha-core/src /app/rinha-core/src
COPY rinha-touche/src /app/rinha-touche/src
RUN touch /app/rinha-core/src/lib.rs
RUN touch /app/rinha-touche/src/main.rs
RUN cargo build --release -p rinha-touche

FROM debian:buster-slim

COPY --from=build /app/target/release/rinha-touche /app/rinha

CMD "/app/rinha"
