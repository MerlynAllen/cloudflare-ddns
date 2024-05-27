FROM ubuntu:22.04
LABEL version="1.0"

ADD target/release/cloudflare-ddns /ddns/cloudflare-ddns
WORKDIR /ddns
ENTRYPOINT [ "/ddns/cloudflare-ddns" ]
