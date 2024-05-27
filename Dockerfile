FROM ubuntu:22.04
LABEL version="1.0"

ADD target/aarch64-unknown-linux-gnu/release/cloudflare-ddns /ddns/cloudflare-ddns
WORKDIR /ddns
ENTRYPOINT [ "/ddns/cloudflare-ddns" ]
