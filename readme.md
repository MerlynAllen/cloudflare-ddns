# Cloudflare DDNS Rust

A simple DDNS tool written in Rust, which updates your domain name on Cloudflare with your current IP.   
Though we have [a Python project](https://github.com/timothymiller/cloudflare-ddns) doing exactly the same thing, there
are some circumstances we prefer a compiled binary
than a script which requires an interpreter,
e.g. a router with limited memory & disk space.

## Build

```bash
cargo build --release
```

If you want to cross-compiling and distribute a binary to a foreign architecture, it is recommended to compile it with
static-linking.

```bash
RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --target aarch64-unknown-linux-gnu
```
## Docker

Currently only available on `Linux x86-64` and `Linux aarch64`.
```bash
docker run -d \
       -v <path-to-config>:/ddns/ddns_config.json \
       --network host \
       --name ddns \
       merlynallen/cloudflare-ddns:latest
```

## Usage

- One-shot
    ```bash
    ./cloudflare-ddns --config <config-path> --oneshot
    ```
- Debug
  ```bash
  ./cloudflare-ddns --config <config-path> --loglevel debug
  ```
- Periodic Task
    
  This program now supports scheduling periodic tasks. But if you want, you can also use crontab.

  Add this to crontab
    ```crontab
    */5 * * * * /path/to/cloudflare-ddns --config /path/to/config --oneshot
    ```
  


## Sample Config File

```
{
  "cf_key": "your_global_api_key",
  "cf_mail": "your_account_email_address",
  "timeout": 10,                           // optional
  "ip_refresh_interval": 300,              // optional
  "domains": [
    {
      "name": "domain.zone",
      "id": "id_of_this_record",
      "update_interval": 600, // optional
      "zone_id": "domain_of_this_zone",
      "record_type": "A",
      "ttl": 60
    },
    {
      "name": "domain.zone",
      "id": "id_of_this_record",
      "update_interval": 600,              // optional
      "zone_id": "domain_of_this_zone",
      "record_type": "AAAA",
      "ttl": 60
    }
  ]
}

```

## Cloudflare API Key

You can get your `Global API Key` from [here](https://dash.cloudflare.com/profile/api-tokens).  
Accessing Cloudflare API using `User API Tokens` is not implemented yet, so User API Tokens are currently unavailable.

## DNS Record ID

Use this command to get the detailed info of your DNS zone.

```bash
curl --request GET \                        
  --url https://api.cloudflare.com/client/v4/zones/<your-zone-id>/dns_records \
  --header 'Content-Type: application/json' \
  --header 'X-Auth-Email: <your-mail>' \
  --header 'X-Auth-Key: <your-api-key>'
```

Find your id info from the json response.
If you have not created your DNS record yet, please create a new one with arbitrary IP, and get its id.  
This will be done automatically in future versions.