FROM rustembedded/cross:aarch64-unknown-linux-gnu

RUN dpkg --add-architecture arm64 && \
    apt-get update && \
    apt-get install --assume-yes \
        libudev-dev:arm64

ENV PKG_CONFIG_PATH /usr/lib/aarch64-linux-gnu/pkgconfig
