#!/bin/bash

set -o errexit
set -o pipefail

cargo install cross
git submodule init
git submodule update

DOCKER_DIR=docker
CROSS_DIR=cross/docker

cp -vf $CROSS_DIR/common.sh       $DOCKER_DIR
cp -vf $CROSS_DIR/lib.sh          $DOCKER_DIR
cp -vf $CROSS_DIR/cmake.sh        $DOCKER_DIR
cp -vf $CROSS_DIR/xargo.sh        $DOCKER_DIR
cp -vf $CROSS_DIR/qemu.sh         $DOCKER_DIR
cp -vf $CROSS_DIR/dropbear.sh     $DOCKER_DIR
cp -vf $CROSS_DIR/linux-image.sh  $DOCKER_DIR
cp -vf $CROSS_DIR/linux-runner    $DOCKER_DIR

cd $DOCKER_DIR

sed -i "s/ubuntu \/etc\/os-release/debian \/etc\/os-release/g" *.sh

docker build -t cross/rpi/4b -f Dockerfile.4b .
docker build -t cross/rpi/zero -f Dockerfile.zero .
