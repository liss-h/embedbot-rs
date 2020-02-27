# embedbot-rs

## Introduction

> This is a discord bot that will embed
- 9GAG videos  
- imgur images   
- reddit images  
- reddit text only posts  
- reddit text posts that are an imgur link  
> It will not embed
- 9GAG images (since discord embeds them just fine)  
- reddit videos (since i have yet to figure out how to embed .m3u8 files)  

## Problems
> This project is under active development (whenever i am bored) and may have bugs, they should mostly appear when embedding from 9GAG since 9GAG does not provide a public API to view post metadata, therefore this bot basically parses JSON inside a javascript script on the posts page, which is not optimal but the best solution i have found yet.

## Dependencies
- rust (nightly) (features: try_trait, bind_by_move_pattern_guards)
- openssl-devel (name may deviate on some distros)
- scraper
- reqwest
- serde_json
- serenity

## Configuration
> define the environment variable DISCORD_TOKEN as your bot token  

## Docker install
> $ wget https://raw.githubusercontent.com/Clueliss/embedbot-rs/master/Dockerfile  
> $ docker build .  
> $ docker run -d -e DISCORD_TOKEN=<YOUR_TOKEN> --name=embedbot <ID_GIVEN_BY_PREV_CMD> 

## Manual build
> $ cargo build --release
