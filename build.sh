#!/bin/bash

set -o errexit
set -o pipefail

if [ "$1" == "pi5" ]; then
  RUSTC_TARGET=aarch64-unknown-linux-gnu
elif [ "$1" == "zero" ]; then
  RUSTC_TARGET=arm-unknown-linux-gnueabi
else
  echo "unknown target: $1"
  exit 1
fi

cross build --release --target $RUSTC_TARGET
