FROM docker.io/fedora

ENV DISCORD_TOKEN=YOUR_DISCORD_TOKEN_HERE

RUN dnf update --refresh -y
RUN dnf install gcc git openssl-devel -y

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > /tmp/rustup.sh
RUN sh /tmp/rustup.sh -y --default-toolchain=nightly

RUN git clone https://github.com/Clueliss/embedbot-rs /tmp/embedbot-rs

RUN cp /tmp/embedbot-rs/update.sh /update
RUN chmod +x /update

RUN /update

CMD ["/init"]
