extern crate c_ares_resolver;
extern crate tokio_core;
extern crate futures;

use std::sync::mpsc as std_mpsc;
use std::thread;
use futures::sync::mpsc;
use futures::{Stream, Future};

const QUERIES: usize = 1000;
static DOMAIN: &'static str = "google.com";

fn main() {
    // Create the request channel. Lookup requests are sent on `tx`, and `rx`
    // provides a stream of those requests.
    let (tx, rx) = mpsc::unbounded::<&'static str>();

    // Channel for results
    let (res_tx, res_rx) = std_mpsc::channel();

    // Spawn a thread to run c-ares on. tokio-core is used as a driver.
    thread::spawn(move || {
        let mut event_loop = tokio_core::reactor::Core::new().unwrap();
        let resolver = c_ares_resolver::FutureResolver::new().unwrap();

        let stream = rx
            // Map each request into a future that should return the lookup
            // result
            .map(|req| {
                resolver
                    // Creates the request
                    .query_a(&req[..])
                    // Transform into a future that is always successful with Item type of
                    // Result<c_ares::AResults, c_ares::Error>
                    .then(|res| Ok(res))
            })
            // Limit how many futures execute in parallel
            .buffer_unordered(10)
            // Send each response on the result channel
            .for_each(|res| {
                let _ = res_tx.send(res);
                Ok(())
            });

        let _ = event_loop.run(stream);
    });

    // Do QUERIES lookups of DOMAIN
    for _ in 0..QUERIES {
        tx.send(DOMAIN).unwrap();
    }

    // Wait for QUERIES responses
    for _ in 0..QUERIES {
        match res_rx.recv() {
            Ok(Ok(res)) => println!("OK: {}", res),
            Ok(Err(err)) => println!("LOOKUP ERR: {}", err),
            Err(err) => println!("CHANNEL ERR: {}", err)
        };
    }
}
