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
    let addr = IpAddr::from_str(&matches.free[0]).unwrap();

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
