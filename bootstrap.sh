#!/bin/bash

set -o errexit
set -o pipefail

cargo install cross

DOCKER_DIR=docker
IMAGE_PREFIX=cross/dht-logger

rpi_hw=(pi5 zero)
for rpi in ${rpi_hw[@]}; do
  docker build -t $IMAGE_PREFIX/$rpi $DOCKER_DIR/$rpi
done
