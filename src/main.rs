extern crate hyper;
extern crate futures;
extern crate trust_dns_resolver;

use trust_dns_resolver::Resolver;
use trust_dns_resolver::lookup::TxtLookup;

use futures::future::Future;

use hyper::header::{Location, Host};
use hyper::server::{Http, Request, Response, Service};
use hyper::StatusCode;

struct RedirectService;

impl Service for RedirectService {
    // boilerplate hooking up hyper's server types
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    // The future representing the eventual Response your call will
    // resolve to. This can change to whatever Future you need.
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let headers = req.headers();
        let result = headers.get::<Host>().and_then(|host: &Host| {
            let resolver = Resolver::default().unwrap();
            resolver.txt_lookup(&format!("_redirect.{}", host.hostname())).ok()
        }).and_then(|txt: TxtLookup| {
            txt.iter().find(|_item| {
                // TODO: actually search for a well formatted redirect
                true
            }).and_then(|txts| {
                let mut acc = Vec::new();
                for value in txts.txt_data() {
                    acc.extend_from_slice(value);
                }
                String::from_utf8(acc).ok()
            })
        }).map(|line| {
            Box::new(futures::future::ok(
                Response::new()
                    .with_header(Location::new(line)).with_status(StatusCode::MovedPermanently)
            ))
        });
        match result {
            None => Box::new(futures::future::ok(
                        Response::new().with_status(StatusCode::NotFound)
                    )),
            Some(x) => x,
        }
    }
}

fn main() {
    let addr = "127.0.0.1:3000".parse().unwrap();
    let server = Http::new().bind(&addr, || Ok(RedirectService)).unwrap();
    server.run().unwrap();
}
