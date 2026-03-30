#!/bin/bash

docker run -it --rm \
    -v .:/cache-cat \
    -v /usr/local/cargo/registry:/usr/local/cargo/registry \
    -w /cache-cat \
    -e RUSTUP_DIST_SERVER=https://mirrors.ustc.edu.cn/rust-static \
    -e RUSTUP_UPDATE_ROOT=https://mirrors.ustc.edu.cn/rust-static/rustup \
    -e DEB_MIRROR=mirrors.ustc.edu.cn \
    ghcr.io/rust-cross/cargo-zigbuild \
    /cache-cat/scripts/build-in-container.sh \
    $@
