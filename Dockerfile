FROM rust:1-slim-buster AS build

RUN cargo new --bin app
WORKDIR /app

COPY Cargo.toml /app/
COPY Cargo.lock /app/
COPY .cargo /app/.cargo
RUN cargo build --release

COPY src /app/src
RUN touch src/main.rs
RUN cargo build --release

FROM debian:buster-slim

COPY --from=build /app/target/release/rinha /app/rinha

CMD "/app/rinha"
