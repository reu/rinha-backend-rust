FROM rust:1-slim-buster AS build

RUN cargo new --bin app
WORKDIR /app
RUN cargo new --lib rinha-core
RUN cargo new --bin rinha-axum
RUN cargo new --lib rinha-touche
RUN mv /app/src/main.rs /app/rinha-axum/src/main.rs

COPY Cargo.toml /app/
COPY Cargo.lock /app/
COPY .cargo /app/.cargo
COPY rinha-core/Cargo.toml /app/rinha-core/
COPY rinha-touche/Cargo.toml /app/rinha-touche/
COPY rinha-axum/Cargo.toml /app/rinha-axum/
RUN cargo build --release -p rinha-axum

COPY rinha-core/src /app/rinha-core/src
COPY rinha-axum/src /app/rinha-axum/src
COPY rinha-axum/.sqlx /app/rinha-axum/.sqlx
RUN touch /app/rinha-core/src/lib.rs
RUN touch /app/rinha-axum/src/main.rs
RUN cargo build --release -p rinha-axum

FROM debian:buster-slim

COPY --from=build /app/target/release/rinha-axum /app/rinha

CMD "/app/rinha"
