// use std::thread;
// use std::time::Duration;
use std::{sync::{Arc, Mutex}, thread, time::Duration};
use clap::Parser;
use parse_duration::{parse as parse_duration};
use reqwest::blocking::Client;
use reqwest::Method;
use reqwest::Url;
// use get_if_addrs::get_if_addrs;
use get_if_addrs::{get_if_addrs, IfAddr};

#[derive(Parser, Debug, Clone)]
#[command(author, version)]
struct Args {
    #[arg(short, long)]
    url: Url,
    #[arg(long, default_value = "GET")]
    method: Method,
    #[arg(long, default_value = "60s", value_parser = parse_duration)]
    interval: Duration,
    #[arg(long, default_value = "false")]
    verbose: bool,
    #[arg(long)]
    interface: Option<String>, // specify the network interface for the IP message
}

fn get_interface_address(interface_name: &str) -> Option<String> {
    if let Ok(if_addrs) = get_if_addrs() {
        for if_addr in if_addrs {
            // eprintln!("{}", if_addr.name);
            if if_addr.name == interface_name {
                if let IfAddr::V4(v4_addr) = if_addr.addr {
                    return Some(v4_addr.ip.to_string());
                } else {
                    eprintln!("Error: Interface '{}' is not an IPv4 address", interface_name);
                    return None;
                }
                // return Some(if_addr.addr.ip().to_string());
            }
        }
    }
    None
}

fn main() {
    let args = Args::parse();
    let client = Client::new();
    let prev_ip_address: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    println!("swatchdog v{} started!", env!("CARGO_PKG_VERSION"));

    loop {
        let client = client.clone();
        let args = args.clone();
        let prev_ip_address_clone = Arc::clone(&prev_ip_address);

        thread::spawn(move || {
            let mut url = args.url.clone();
            let mut status = "up";

            // If the interface name is provided, fetch its IPv4 global address
            if let Some(interface_name) = &args.interface {
                if let Some(interface_address) = get_interface_address(interface_name) {
                    let mut prev_ip = prev_ip_address_clone.lock().unwrap();
                    if let Some(prev_ip_str) = &*prev_ip {
                        // Compare with the previous IP address
                        if prev_ip_str != &interface_address {
                            status = "pending";
                        }
                        else {
                            status = "up";
                        }
                    }
                    let query = format!("status={}&msg={}&ping=", status, interface_address);
                    url.set_query(Some(&query));

                    // Update the previous IP address
                    *prev_ip = Some(interface_address.clone());
                } else {
                    eprintln!("Error: Unable to retrieve IPv4 address for interface '{}'", interface_name);
                    // return;
                }
            }
            
            if args.verbose {
                eprint!("Send request to {} ... ", url);
            }

            let result = client.request(args.method, url).send().and_then(|res| res.error_for_status());

            if let Err(err) = result {
                eprintln!("Error: {}", err)
            }

            if args.verbose {
                eprintln!("Success");
            }
        });

        thread::sleep(args.interval);
    }
}