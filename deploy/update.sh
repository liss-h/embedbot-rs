#!/bin/bash

set -e

cd /tmp

if cd ./embedbot-rs; then
    git pull
else
    git clone https://github.com/Clueliss/embedbot-rs
    cd ./embedbot-rs
fi

cargo build --release

rm /init || true
cp ./target/release/embedbot-rs /init
chmod +x /init

rm /update || true
cp ./deploy/update.sh /update
chmod +x /update

rm /system-update || true
cp ./deploy/system-update.sh /system-update
chmod +x /system-update
