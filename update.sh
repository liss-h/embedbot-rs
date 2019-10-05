#!/bin/bash

set -e

cd /tmp

if cd ./embedbot-rs; then
    git pull
else
    git clone https://github.com/Clueliss/embedbot-rs
    cd ./embedbot-rs
fi

/root/.cargo/bin/cargo build --release

rm /usr/bin/embedbot-rs
cp ./target/release/embedbot-rs /usr/bin
chmod +x /usr/bin/embedbot-rs
