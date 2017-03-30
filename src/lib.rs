extern crate c_ares_resolver;
extern crate c_ares;
extern crate tokio_core;
extern crate futures;

use std::borrow::Cow;
use std::sync::mpsc as std_mpsc;
use std::net::IpAddr;
use std::thread;

use c_ares::{AResults, AAAAResults};

use futures::sync::mpsc;
use futures::{Stream, Future};

pub struct Dns {
    tx: mpsc::UnboundedSender<Cow<'static, str>>,
}

type ResolveResult = (c_ares::Result<AResults>, c_ares::Result<AAAAResults>);

/// Converts the ResolveResult into a vector of IpAddr
///
/// the ENODATA error is essentially ignored. If both queries returned ENODATA,
/// the result of this function will be Ok with an empty Vec.
fn responses_into_iter(responses: ResolveResult) -> c_ares::Result<Vec<IpAddr>> {
    let mut addrs = Vec::new();

    let (a_result, aaaa_result) = responses;

    match aaaa_result {
        Ok(aaaa) => {
            for entry in aaaa.iter() {
                addrs.push(entry.ipv6().into());
            }
        },
        Err(c_ares::Error::ENODATA) => (),
        Err(err) => return Err(err),
    }

    match a_result {
        Ok(a) => {
            for entry in a.iter() {
                addrs.push(entry.ipv4().into());
            }
        },
        Err(c_ares::Error::ENODATA) => (),
        Err(err) => return Err(err),
    }

    Ok(addrs)
}

impl Dns {
    pub fn resolve<S>(&self, domain: S) -> Result<(), mpsc::SendError<Cow<'static, str>>>
        where S: Into<Cow<'static, str>>
    {
        self.tx.send(domain.into())
    }

    pub fn new<F>(callback: F) -> Dns
        where F: Fn(c_ares::Result<Vec<IpAddr>>) + Send + 'static
    {
        // Create the request channel. Lookup requests are sent on `tx`, and `rx`
        // provides a stream of those requests.
        let (tx, rx) = mpsc::unbounded::<Cow<'static, str>>();

        // Spawn a thread to run c-ares on. tokio-core is used as a driver.
        thread::spawn(move || {
            let mut event_loop = tokio_core::reactor::Core::new().unwrap();
            let resolver = c_ares_resolver::FutureResolver::new().unwrap();

            let stream = rx
                // Map each request into a future that should return the lookup
                // result
                .map(|req| {
                    let a_query = resolver
                        // Creates the A request
                        .query_a(&req[..])
                        // Transform into a future that is always successful with Item type of
                        // Result<c_ares::AResults, c_ares::Error>
                        .then(|res| Ok(res));

                    let aaaa_query = resolver
                        // Create the AAAA request
                        .query_aaaa(&req[..])
                        // Transform into a future that is always successful with Item type of
                        // Result<c_ares::AAAAResults, c_ares::Error>
                        .then(|res| Ok(res));

                    a_query.join(aaaa_query)
                })
                // Limit how many futures execute in parallel
                .buffer_unordered(25)
                // Send each response on the result channel
                .for_each(|res| {
                    callback(responses_into_iter(res));
                    Ok(())
                });

            let _ = event_loop.run(stream);
        });

        Dns { tx: tx }
    }
}