#!/bin/bash

DOCKER_TARGETS="linux/amd64 linux/arm64 linux/riscv64"

./scripts/build-multitarget.sh ${DOCKER_TARGETS}

docker buildx build --platform $(echo $DOCKER_TARGETS| tr ' ' ',') -t cache-cat:latest  .