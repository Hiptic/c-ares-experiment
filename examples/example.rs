extern crate c_ares_experiment;

use std::sync::mpsc;

use c_ares_experiment::Dns;

const QUERIES: usize = 1000;

fn main() {
    // Channel for results
    let (tx, rx) = mpsc::channel();

    let dns = Dns::new(move |res| tx.send(res).unwrap());

    // Request QUERIES lookups
    for _ in 0..QUERIES {
        let _ = dns.resolve("google.com");
    }

    // Wait for QUERIES responses
    for _ in 0..QUERIES {
        match rx.recv() {
            Ok((host, Ok(addrs))) => println!("OK [{}]: {:?}", host, addrs),
            Ok((host, Err(err))) => println!("DNS ERR [{}]: {}", host, err),
            Err(err) => println!("CHANNEL ERR: {}", err)
        };
    }
}
