#[macro_use] extern crate log;
extern crate env_logger;
extern crate hyper;
extern crate futures;
extern crate trust_dns_resolver;
extern crate tokio_core;

use trust_dns_resolver::ResolverFuture;
use trust_dns_resolver::config::*;
use trust_dns_resolver::lookup::TxtLookup;

use futures::future::Future;
use futures::Stream;

use hyper::header::{Location, Host};
use hyper::server::{Http, Request, Response, Service};
use hyper::StatusCode;

use std::env;

const NOTFOUND: &str = "404 Not found";

struct RedirectService(tokio_core::reactor::Handle);

impl Service for RedirectService {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let host = match req.headers().get::<Host>() {
            None => return Box::new(futures::future::ok(Response::new().with_status(StatusCode::BadRequest).with_body("No Host header"))),
            Some(x) => x,
        };

        let resolver = ResolverFuture::new(ResolverConfig::default(), ResolverOpts::default(), &self.0);
        let name = format!("_redirect.{}", host.hostname());
        let failure = format!("{} => Not found", host.hostname());

        Box::new(resolver.txt_lookup(name.clone()).map(move |txt: TxtLookup| {
            let line: Option<String> = txt.iter().next().and_then(|txts| {
                let mut acc = Vec::new();
                for value in txts.txt_data() {
                    acc.extend_from_slice(value);
                }
                String::from_utf8(acc).ok()
            });
            match line {
                None => Response::new().with_status(StatusCode::NotFound).with_body(NOTFOUND),
                Some(x) => {
                    info!("{} => {}", name, x);
                    Response::new().with_header(Location::new(x)).with_status(StatusCode::MovedPermanently)
                },
            }
        }).or_else(move |_err| {
            info!("{}", failure);
            futures::future::ok(Response::new().with_status(StatusCode::NotFound).with_body(NOTFOUND))
        }))
    }
}

fn main() {
    env_logger::init();

    let addr_str = match env::var("LISTEN_ADDR") {
        Err(_) => { eprintln!("You must supply the LISTEN_ADDR environment variable"); return; },
        Ok(x) => x,
    };

    let addr = match addr_str.parse() {
        Err(_) => { eprintln!("Unable to parse LISTEN_ADDR "); return; },
        Ok(x) => x,
    };

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let server_handle = core.handle();
    let client_handle = core.handle();

    let serve = Http::new().serve_addr_handle(&addr, &server_handle, move || Ok(RedirectService(client_handle.clone()))).unwrap();
    info!("Listening on http://{} with 1 thread.", serve.incoming_ref().local_addr());

    let h2 = server_handle.clone();
    server_handle.spawn(serve.for_each(move |conn| {
        h2.spawn(conn.map(|_| ()).map_err(|err| println!("serve error: {:?}", err)));
        Ok(())
    }).map_err(|_| ()));

    core.run(futures::future::empty::<(), ()>()).unwrap();
}
