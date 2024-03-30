# embedbot-rs

A discord bot that creates embeds for things discord does not properly embed itself. Currently supported:
- 9GAG posts  
- Imgur images   
- reddit posts
- SVGs
- Tweets

## Configuration
See `config/init.json` and `config/runtime.json`

## Docker install
```shell
$ wget https://raw.githubusercontent.com/Clueliss/embedbot-rs/master/Dockerfile  
$ docker build --tag embedbot-rs .  
$ docker run -d --name=embedbot \
    -v ./init.json:/etc/embedbot/init.json:ro \
    -v ./runtime.json:/etc/embedbot/runtime.json \
    embedbot-rs
```

## Manual build
```shell
$ cargo build --release
```
