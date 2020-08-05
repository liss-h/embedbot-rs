#!/bin/bash

set -e

dnf update --refresh -y
rustup update

/update
