# embedbot-rs

A discord bot that creates embeds for things discord does not properly embed itself. Currently supported:
- 9GAG posts
- reddit posts
- SVGs
- Tweets

## Configuration
See [embedbot.json](embedbot.json)

## Docker install
```shell
$ wget https://raw.githubusercontent.com/Clueliss/embedbot-rs/master/Dockerfile  
$ docker build --tag embedbot-rs .  
$ docker run -d --name=embedbot \
    -v ./embedbot.json:/etc/embedbot.json:ro \
    embedbot-rs
```

## Manual build
```shell
$ cargo build --release
```
