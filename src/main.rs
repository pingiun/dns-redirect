#[macro_use] extern crate log;
extern crate pretty_env_logger;
extern crate hyper;
extern crate futures;
extern crate trust_dns_resolver;
extern crate tokio_core;
extern crate regex;

use regex::Regex;

use trust_dns_resolver::AsyncResolver;
use trust_dns_resolver::lookup::TxtLookup;
use trust_dns_resolver::config::*;

use futures::{future, Future};

use hyper::{Body, Response, Request, StatusCode, Server};
use hyper::header::{HOST};
use hyper::service::service_fn;

use std::env;

const NOTFOUND: &str = "404 Not found";

fn match_status(status: &str) -> StatusCode {
    match status {
        "301" | "moved" => StatusCode::MOVED_PERMANENTLY,
        "302" | "found" => StatusCode::FOUND,
        "303" | "see_other" => StatusCode::SEE_OTHER,
        "307" | "temporary" => StatusCode::TEMPORARY_REDIRECT,
        "308" | "permanent" => StatusCode::PERMANENT_REDIRECT,
        _ => StatusCode::MOVED_PERMANENTLY,
    }
}

fn parse_rewrite(url: &str, parts: Vec<&str>) -> (String, StatusCode) {
    debug!("Parsing rewrite command");
    if parts.len() < 3 || parts.len() > 4 {
        debug!("Invalid record");
        return (parts.join(" "), StatusCode::MOVED_PERMANENTLY);
    }
    let re = match Regex::new(parts[1]) {
        Ok(x) => x,
        Err(_) =>  {
            debug!("Input doesn't match regex");
            return (parts.join(" "), StatusCode::MOVED_PERMANENTLY);
        },
    };
    debug!("Replacing {} with match {} into {}", url, parts[1], parts[2]);
    let location = re.replace(url, parts[2]).into();
    (location, if parts.len() == 4 {
        match_status(parts[3])
    } else {
        StatusCode::MOVED_PERMANENTLY
    })
}

fn get_location_url(url: &str, line: &str) -> (String, StatusCode) {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() == 0 {
        return (line.to_string(), StatusCode::MOVED_PERMANENTLY);
    }
    match parts[0] {
        "rewrite" => parse_rewrite(url, parts),
        _ => (line.to_string(), StatusCode::MOVED_PERMANENTLY)
    }
}

fn redirector_response(req: Request<Body>, client: &AsyncResolver) 
    -> Box<Future<Item=Response<Body>, Error=hyper::Error> + Send> 
{
    let host = match req.headers().get(HOST) {
        None => return Box::new(futures::future::ok(Response::builder().status(StatusCode::BAD_REQUEST).body(Body::from("No Host header")).unwrap())),
        Some(x) => x,
    };

    let name = format!("_redirect.{}", host.to_str().unwrap());
    let failure = format!("{} => Not found", host.to_str().unwrap());
    let path = req.uri().path().to_string();

    Box::new(client.txt_lookup(name.clone()).map(move |txt: TxtLookup| {
        let line = txt.iter().next().and_then(|txts| {
            let mut acc = Vec::new();
            for value in txts.txt_data() {
                acc.extend_from_slice(value);
                acc.extend_from_slice(&[' ' as u8]);
            }
            acc.pop();
            String::from_utf8(acc).ok()
        });
        match line {
            None => Response::builder().status(StatusCode::NOT_FOUND).body(Body::from(NOTFOUND)).unwrap(),
            Some(x) => {
                let (newurl, code) = get_location_url(&path, &x);
                info!("{} => {}", name, newurl);
                Response::builder().header("Location", newurl).status(code).body(Body::from("")).unwrap()
            },
        }
    }).or_else(move |_err| {
        info!("{}", failure);
        futures::future::ok(Response::builder().status(StatusCode::NOT_FOUND).body(Body::from(NOTFOUND)).unwrap())
    }))
}

fn main() {
    pretty_env_logger::init();

    let addr_str = match env::var("LISTEN_ADDR") {
        Err(_) => { error!("You must supply the LISTEN_ADDR environment variable"); return; },
        Ok(x) => x,
    };

    let addr4 = match addr_str.parse() {
        Err(_) => { error!("Unable to parse LISTEN_ADDR "); return; },
        Ok(x) => x,
    };

    let addr6 = match env::var("LISTEN_ADDR_6") {
        Err(_) => None,
        Ok(x) => x.parse().ok(),
    };

    hyper::rt::run(future::lazy(move || {
        let (client4, background) = AsyncResolver::new(ResolverConfig::default(), ResolverOpts::default());
        let client6 = client4.clone();
        hyper::rt::spawn(background);

        let service4 = move || {
            let client = client4.clone();
            service_fn(move |req| {
                redirector_response(req, &client)
            })
        };

        let server4 = Server::bind(&addr4)
            .serve(service4)
            .map_err(|e| error!("server error {}", e));
        hyper::rt::spawn(server4);
        
        info!("Listening on http://{}", addr4);
        
        if let Some(addr6) = addr6 {
            let service6 = move || {
                let client = client6.clone();
                service_fn(move |req| {
                    redirector_response(req, &client)
                })
            };
            let server6 = Server::bind(&addr6)
                .serve(service6)
                .map_err(|e| error!("server error {}", e));
            hyper::rt::spawn(server6);
            info!("Listening on http://{}", addr6)
        };
        Ok(())
    }));
}
