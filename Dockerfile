FROM docker.io/fedora

ENV DISCORD_TOKEN=""
ENV PATH="/root/.cargo/bin:${PATH}"

RUN dnf update --refresh -y && \
    dnf install gcc git openssl-devel -y && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > /tmp/rustup.sh && \
    sh /tmp/rustup.sh -y --default-toolchain=nightly && \
    mkdir /tmp/embedbot-rs

COPY . /tmp/embedbot-rs

RUN cp /tmp/embedbot-rs/deploy/update.sh /update && \
    cp /tmp/embedbot-rs/deploy/system-update.sh /system-update && \
    chmod +x /update && \
    chmod +x /system-update && \
    /update

CMD ["/init", "--settings-file", "/etc/embedbot.conf"]
