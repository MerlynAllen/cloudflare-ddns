FROM alpine:3.9.6

ARG ARCH=x86_64
ARG LIBC=musl
ARG VERSION=1.0.0

WORKDIR /ddns

ADD https://github.com/MerlynAllen/cloudflare-ddns/releases/download/v${VERSION}/cloudflare-ddns-Linux-${LIBC}-${ARCH}.tar.gz /ddns/

RUN tar zvxf cloudflare-ddns-Linux-${LIBC}-${ARCH}.tar.gz && \
    rm cloudflare-ddns-Linux-${LIBC}-${ARCH}.tar.gz && \
    chmod +x /ddns/cloudflare-ddns

ENTRYPOINT [ "/ddns/cloudflare-ddns" ]
