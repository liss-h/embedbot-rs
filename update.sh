#!/bin/bash

set -e

dnf update --refresh -y
/root/.cargo/bin/rustup update

cd /tmp

if cd ./embedbot-rs; then
    git pull
else
    git clone https://github.com/Clueliss/embedbot-rs
    cd ./embedbot-rs
fi

/root/.cargo/bin/cargo build --release

rm /init || true
cp ./target/release/embedbot-rs /init
chmod +x /init

rm /update || true
cp ./update.sh /update
chmod +x /update
