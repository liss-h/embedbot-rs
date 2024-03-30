FROM rust:1-slim-bookworm AS base
RUN cargo install cargo-chef


FROM base as planner

WORKDIR /usr/local/src/embedbot-rs
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY src src

RUN cargo chef prepare --recipe-path recipe.json


FROM base as builder

WORKDIR /usr/local/src/embedbot-rs

COPY --from=planner /usr/local/src/embedbot-rs/recipe.json recipe.json
RUN cargo chef cook --recipe-path recipe.json --release

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY src src
RUN cargo build --release


FROM debian:bookworm-slim

RUN apt-get update && apt-get upgrade && apt-get install ca-certificates chromium -y

COPY ./embedbot.json /etc/embedbot.json
COPY --from=builder /usr/local/src/embedbot-rs/target/release/embedbot-rs /usr/local/bin/

RUN chmod +x /usr/local/bin/embedbot-rs

ENTRYPOINT ["/usr/local/bin/embedbot-rs"]
