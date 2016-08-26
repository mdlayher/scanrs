extern crate getopts;

use getopts::Options;
use std::env;
use std::io::{self, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpStream};
use std::str::FromStr;
use std::sync::mpsc::{Sender, channel};
use std::thread;

// Max possible port
const MAX: u16 = 65535;

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");
    opts.optopt("j",
                "",
                "number of threads to use for concurrent scanning",
                "THREADS");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            println!("error parsing options: {}", e.to_string());
            return;
        }
    };

    // Present usage if '-h' or no IP address provided
    if matches.opt_present("h") || matches.free.is_empty() {
        let brief = format!("Usage: {} -j THREADS IPADDR", program);
        print!("{}", opts.usage(&brief));
        return;
    }

    // Default to 4 threads, but allow user to specify more or less
    let num_threads = match matches.opt_str("j") {
        Some(j) => j.parse().expect("flag '-j' must be an integer"),
        None => 4,
    };

    // Parse IPv4 or IPv6 address for scanning
    let addr = match parse_ip(&matches.free[0]) {
        Ok(ip) => ip,
        Err(e) => {
            println!("error parsing IP address: {}", e.to_string());
            return;
        }
    };

    // Send and receive results via channels of port numbers, scanning
    // concurrently using threads
    let (tx, rx) = channel::<u16>();
    for i in 0..num_threads {
        let tx = tx.clone();

        thread::spawn(move || {
            scan(tx, i, addr, num_threads);
        });
    }

    // Drop transmit side of channel in main thread so that for loop
    // will end once all worker threads complete
    let mut out = vec![];
    drop(tx);
    for port in rx {
        out.push(port);
    }

    // Add newline after progress, sort vector and print all ports
    println!("");
    out.sort();
    for v in out {
        println!("{} is open", v);
    }
}

// scan scans ports at an IP address and sends any open ports it finds back on its
// channel.  scan exits once MAX has been reached.
fn scan(tx: Sender<u16>, start_port: u16, addr: IpAddr, num_threads: u16) {
    let mut port: u16 = start_port + 1;

    loop {
        match TcpStream::connect((addr, port)) {
            Ok(_) => {
                // Found open port, indicate progress and send to main thread
                print!(".");
                io::stdout().flush().unwrap();
                tx.send(port).unwrap();
            }
            Err(_) => {}
        }

        // Break loop when out of ports
        if (MAX - port) <= num_threads {
            break;
        }

        port += num_threads;
    }
}

// parse_ip attempts to parse an input string as an IPv4 or IPv6 address.
fn parse_ip(addr: &str) -> Result<IpAddr, String> {
    if let Ok(ip4) = Ipv4Addr::from_str(addr) {
        return Ok(IpAddr::V4(ip4));
    }

    if let Ok(ip6) = Ipv6Addr::from_str(addr) {
        return Ok(IpAddr::V6(ip6));
    }

    Err(format!("could not parse {} as an IP address", addr).to_string())
}

#[test]
fn parse_ip_ok() {
    let test_ip4 = IpAddr::V4(Ipv4Addr::from_str("192.168.1.1").unwrap());
    let test_ip6 = IpAddr::V6(Ipv6Addr::from_str("::1").unwrap());
    let test_bad = "could not parse foobar as an IP address".to_string();

    let ip4 = parse_ip("192.168.1.1").unwrap();
    assert!(ip4 == test_ip4);

    let ip6 = parse_ip("::1").unwrap();
    assert!(ip6 == test_ip6);

    let bad = match parse_ip("foobar") {
        Ok(_) => panic!("foobar is bad input"),
        Err(e) => e,
    };
    assert!(bad == test_bad);
}
