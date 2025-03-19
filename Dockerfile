FROM alpine:3.9.6

ARG ARCH

COPY target/$ARCH/release/cloudflare-ddns /ddns/cloudflare-ddns

WORKDIR /ddns

ENTRYPOINT [ "/ddns/cloudflare-ddns" ]
