extern crate c_ares_resolver;
extern crate tokio_core;
extern crate futures;

use std::sync::mpsc as std_mpsc;
use std::thread;
use futures::sync::mpsc;
use futures::{Stream, Future};

const QUERIES: usize = 1000;

fn main() {

    let (tx, rx) = mpsc::unbounded::<(usize, &'static str)>();
    let (res_tx, res_rx) = std_mpsc::channel();

    thread::spawn(move || {
        let mut event_loop = tokio_core::reactor::Core::new().unwrap();
        let resolver = c_ares_resolver::FutureResolver::new().unwrap();

        let stream = rx.map(|(i, req)| {
            println!("req: {}, {}", i, req);
            resolver.query_a(&req[..])
                .join(Ok(i))
                .map_err(|_| ())
        }).buffer_unordered(5).for_each(|(res, i)| {
            let _ = res_tx.send((i, res));
            Ok(())
        });

        let _ = event_loop.run(stream);
    });

    for i in 0..QUERIES {
        tx.send((i, "google.com")).unwrap();
    }

    for _ in 0..QUERIES {
        let (i, res) = match res_rx.recv() {
            Ok(v) => v,
            Err(err) => panic!("{}", err)
        };
        println!("{}, {}", i, res);
    }
}
