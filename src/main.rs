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
    let (tx, rx) = mpsc::unbounded::<(usize, &'static str)>();

    // Channel for results
    let (res_tx, res_rx) = std_mpsc::channel();

    // Spawn a thread to run c-ares on. tokio-core is used as a driver.
    thread::spawn(move || {
        let mut event_loop = tokio_core::reactor::Core::new().unwrap();
        let resolver = c_ares_resolver::FutureResolver::new().unwrap();

        let stream = rx
            // Map each request into a future that should return the lookup
            // result
            .map(|(i, req)| {
                println!("req: {}, {}", i, req);
                resolver.query_a(&req[..])
                    .join(Ok(i))
                    .map_err(|_| ()) // errors are.. ignored :(
            })
            // Limit how many futures execute in parallel
            .buffer_unordered(20)
            // Send each response on the result channel
            .for_each(|(res, i)| {
                let _ = res_tx.send((i, res));
                Ok(())
            });

        let _ = event_loop.run(stream);
    });

    // Do QUERIES lookups of DOMAIN
    for i in 0..QUERIES {
        tx.send((i, DOMAIN)).unwrap();
    }

    // Wait for QUERIES responses
    for _ in 0..QUERIES {
        match res_rx.recv() {
            Ok((i, res)) => println!("{}, {}", i, res),
            Err(err) => println!("{}", err)
        };
    }
}
