#![feature(thread_sleep_until)]
#![feature(duration_constructors)]

use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::Path;
use std::sync::{Arc, RwLock, RwLockReadGuard};
use std::time::Duration;

use clap::Parser;
use dns_lookup::getaddrinfo;
use log::{debug, error, info, LevelFilter};
use log::LevelFilter::Debug;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

use IpVersion::*;

use crate::scheduler::Scheduler;

mod scheduler;

#[derive(Debug, Deserialize)]
struct Config {
    cf_key: String,
    cf_mail: String,
    timeout: Option<u64>,
    ip_refresh_interval: Option<u64>,
    domains: Vec<Domain>,
}

#[derive(Debug, Deserialize)]
struct Domain {
    id: String,
    update_interval: u64,
    zone_id: String,
    record_type: RecordType,
    name: String,
    ttl: u64,
}


#[derive(Debug, Parser)]
struct Cmd {
    #[arg(short, long, default_value = "ddns_config.json")]
    config: String,
    #[arg(short, long)]
    oneshot: bool,
    #[arg(short, long, default_value = "Off")]
    loglevel: LevelFilter,
}

#[derive(Debug, Serialize)]
struct RequestBody {
    content: IpAddr,
    name: String,
    proxied: bool,
    #[serde(rename = "type")]
    record_type: RecordType,
    comment: String,
    ttl: u64,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
enum RecordType {
    A,
    #[serde(rename = "AAAA")]
    Aaaa,
}

const DEFAULT_TIMEOUT_SECS: u64 = 10;
const DEFAULT_IP_REFRESH_INTERVAL_SECS: u64 = 300;

const CF_API: &str = "https://api.cloudflare.com/client/v4";
const MYIP_API: &str = "https://myip.merlyn.dev/";
const MYIP_HOST: &str = "myip.merlyn.dev";
// struct DNSResult(Option<Ipv4Addr>, Option<Ipv6Addr>);
static MYIP_DNS_RECORDS: RwLock<Option<Vec<SocketAddr>>> = RwLock::new(None);
static CONFIG: RwLock<Option<Config>> = RwLock::new(None);

fn myip_api_dns_resolve() -> Option<Vec<SocketAddr>> {
    let mut result = Vec::new();
    for addr in getaddrinfo(Some(MYIP_HOST), None, None).ok()?.inspect(|addr_info| debug!("getaddrinfo: {addr_info:?}")) {
        let addr = addr.ok()?.sockaddr;
        result.push(addr)
    }
    Some(result)
}

enum IpVersion {
    V4,
    V6,
}


/// Refresh local cache
fn refresh_ip() -> Option<()> {
    let mut myip_dns_records = MYIP_DNS_RECORDS.write().ok()?;
    debug!("Resolving DNS of {MYIP_HOST}");
    *myip_dns_records = myip_api_dns_resolve();
    Some(())
}

/// Gets IP from local cache, do not refresh
fn get_ip(ip_version: IpVersion) -> Option<IpAddr> {
    let dns_result = { MYIP_DNS_RECORDS.read().ok()?.clone()? };

    let get_ip_with_overwritten_dns = |mut ip: SocketAddr| -> Option<IpAddr> {
        ip.set_port(443);
        debug!("Trying connecting to {}", ip);
        let client = match reqwest::blocking::ClientBuilder::new()
            .use_rustls_tls()
            .resolve(MYIP_HOST, ip)
            .build() {
            Ok(c) => c,
            Err(e) => {
                debug!("Cannot connect: {e:?}");
                return None;
            }
        };
        let response = match client.get(MYIP_API).send() {
            Ok(r) => r,
            Err(e) => {
                debug!("Cannot get response: {e:?}");
                return None;
            }
        };
        if !response.status().is_success() {
            debug!("Request failed with {:?}", response);
            return None;
        }
        let ip_string = response.text().ok()?;
        let ip = match ip_version {
            V4 => IpAddr::V4(ip_string.parse::<Ipv4Addr>().ok()?),
            V6 => IpAddr::V6(ip_string.parse::<Ipv6Addr>().ok()?)
        };
        Some(ip)
    };
    match ip_version {
        V4 => {
            for ipv4 in dns_result.iter().filter(|&i| i.is_ipv4()) {
                match get_ip_with_overwritten_dns(*ipv4) {
                    None => continue,
                    others => return others
                }
            }
            None
        }
        V6 => {
            for ipv6 in dns_result.iter().filter(|&i| i.is_ipv6()) {
                match get_ip_with_overwritten_dns(*ipv6) {
                    None => continue,
                    others => return others
                }
            }
            None
        }
    }
}


fn read_config<T: AsRef<Path>>(path: T) -> Option<Config> {
    let path = path.as_ref().to_owned();
    if !path.exists() {
        return None;
    }
    // Exists, open it
    let config_file = std::fs::read(path).inspect_err(|e| error!("{e:?}")).ok()?;
    serde_json::from_slice(&config_file).inspect_err(|e| error!("{e:?}")).ok()
}

fn compose_headers(config: &Config) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("X-Auth-Key", HeaderValue::from_str(config.cf_key.as_str()).unwrap());
    headers.insert("X-Auth-Email", HeaderValue::from_str(config.cf_mail.as_str()).unwrap());
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));
    headers
}

fn compose_body(domain: &Domain, (ipv4, ipv6): (Option<IpAddr>, Option<IpAddr>)) -> Option<String> {
    let ip = match domain.record_type {
        RecordType::A => ipv4,
        RecordType::Aaaa => ipv6
    }?; // If the IP does not exist, return None.

    let now_string = chrono::offset::Local::now();
    let body = RequestBody {
        content: ip,
        name: domain.name.clone(),
        proxied: false,
        record_type: domain.record_type,
        comment: format!("Updated at {now_string}"),
        ttl: domain.ttl,
    };
    info!("Updating {} to {}", domain.name, ip);
    serde_json::to_string(&body).ok()
}


fn task_refresh_ip() -> Option<()> {
    refresh_ip();
    let ipv4 = get_ip(V4);
    let ipv6 = get_ip(V6);
    info!("IPv4: {}", if let Some(ipv4) = ipv4 { ipv4.to_string() } else { "None".to_string() });
    info!("IPv6: {}", if let Some(ipv6) = ipv6 { ipv6.to_string() } else { "None".to_string() });
    Some(())
}


fn oneshot(config: &Config) {
    task_refresh_ip();
    let ipv4 = get_ip(V4);
    let ipv6 = get_ip(V6);
    let headers = compose_headers(config);
    for domain in config.domains.iter().clone() {
        let body = match compose_body(domain, (ipv4, ipv6)) {
            Some(body) => body,
            None => return,
        };
        let client = reqwest::blocking::ClientBuilder::new()
            .use_rustls_tls()
            .default_headers(headers.clone())
            .timeout(Duration::from_secs(config.timeout.unwrap_or(DEFAULT_TIMEOUT_SECS)))
            .build().unwrap();
        let api = format!("{}/zones/{}/dns_records/{}", CF_API, domain.zone_id, domain.id);
        let response = client
            .patch(api)
            .body(body)
            .send();
        if response.is_err() {
            debug!("{:?}", response.unwrap_err());
            return;
        }
        debug!("{:?}", response.unwrap());
        info!("Done updating.");
        println!("Done.")
    }
}

fn main() {
    let mut logger_builder = pretty_env_logger::formatted_timed_builder();
    let args = Cmd::parse();
    logger_builder.filter_level(args.loglevel).init();
    // read config
    let config = Box::new(read_config(args.config).expect("Error reading config!"));
    let config: &'static _ = Box::leak(config);

    if args.oneshot {
        println!("Oneshot");
        return oneshot(config);
    }

    // {
    //     let mut global_config_lock = CONFIG.write().unwrap();
    //     *global_config_lock = Some(config);
    // } // Drop the write lock
    let mut scheduler = Scheduler::new();

    scheduler.schedule(
        "IP updater".to_string(),
        Arc::new(task_refresh_ip),
        Duration::from_secs(config.ip_refresh_interval.unwrap_or(DEFAULT_IP_REFRESH_INTERVAL_SECS)),
    );


    // let config = CONFIG.read().ok()?;
    let headers = compose_headers(config);
    for domain in config.domains.iter().clone() {
        let headers = headers.clone();
        scheduler.schedule(
            format!("Domain updater ({})", domain.name),
            Arc::new(move || {
                let ipv4 = get_ip(V4);
                let ipv6 = get_ip(V6);
                let body = match compose_body(domain, (ipv4, ipv6)) {
                    Some(body) => body,
                    None => return Some(()),
                };
                let client = reqwest::blocking::ClientBuilder::new()
                    .use_rustls_tls()
                    .default_headers(headers.clone())
                    .timeout(Duration::from_secs(config.timeout.unwrap_or(DEFAULT_TIMEOUT_SECS)))
                    .build().unwrap();
                let api = format!("{}/zones/{}/dns_records/{}", CF_API, domain.zone_id, domain.id);
                let response = client
                    .patch(api)
                    .body(body)
                    .send();
                if response.is_err() {
                    debug!("{:?}", response.unwrap_err());
                    return Some(());
                }
                debug!("{:?}", response.unwrap());
                info!("Done updating.");

                Some(())
            }),
            Duration::from_secs(domain.update_interval),
        );
    };


    scheduler.run();
}
