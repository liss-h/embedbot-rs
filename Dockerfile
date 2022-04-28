FROM rustlang/rust:nightly-bullseye-slim AS builder
WORKDIR /usr/local/src/embedbot-rs
COPY ./Cargo.toml ./
COPY ./src ./src
RUN cargo build --release


FROM debian:bullseye-slim

RUN apt-get update && apt-get install ca-certificates -y
RUN mkdir /etc/embedbot

COPY ./config/runtime.json /etc/embedbot/runtime.json
COPY --from=builder /usr/local/src/embedbot-rs/target/release/embedbot-rs /usr/local/bin/

RUN chmod +x /usr/local/bin/embedbot-rs

ENTRYPOINT ["/usr/local/bin/embedbot-rs"]
