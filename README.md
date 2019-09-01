# embedbot-rs

## Introduction

> rust port of my python version of this bot (embedbot)
> This is a discord bot that will embed
- 9GAG videos 
- reddit images
> It will not embed
- 9GAG images (since discord embeds them just fine)
- reddit videos (since i have yet to figure out how to embed .m3u8 files)

## Problems
> This project is under active development (whenever i am bored) and may have bugs, they should mostly appear when embedding from 9GAG since 9GAG does not provide a public API to view post metadata. This bot basically parses JSON inside a javascript script on the posts page, which is not optimal but the best solution i have found yet.

## Dependencies
- rust (nightly)
- openssl-devel (on Fedora, name may deviate on other distros)
- scraper
- reqwest
- serde_json
- serenity

## Configuration
> define the environment variable DISCORD_TOKEN as your bot token  

## Docker install
> $ docker build embedbot-rs

## Manual build
> $ cargo build --release
