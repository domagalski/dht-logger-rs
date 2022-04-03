FROM rustembedded/cross:armv7-unknown-linux-gnueabihf

RUN dpkg --add-architecture armhf && \
    apt-get update && \
    apt-get install --assume-yes \
        libudev-dev:armhf

ENV PKG_CONFIG_PATH /usr/lib/arm-linux-gnueabihf/pkgconfig
