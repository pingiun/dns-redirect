# Redirector
Rust service for redirecting based on a _redirect. TXT record.

## Who is this for?
If you just bought a domain but have not set up a webserver yet, you can use 
301.systems for redirecting your domain. Some registrars can do this for you,
but you can use 301.systems if you registrar does not.

## How to use the 301.systems service.
1. If your dns provider supports CNAME records for a root domain, set up a 
CNAME to `301.systems`. Otherwise use an A record that points to `35.193.52.78`.

2. Set up a TXT record for _redirect.yourdomain.tld. The TXT record should
contain a valid URL to the site that you want to be redirected to. For example:
```terminal
$ dig TXT _redirect.301.systems

; <<>> DiG 9.9.7-P3 <<>> TXT _redirect.301.systems
;; global options: +cmd
;; Got answer:
;; ->>HEADER<<- opcode: QUERY, status: NOERROR, id: 51808
;; flags: qr rd ra; QUERY: 1, ANSWER: 1, AUTHORITY: 0, ADDITIONAL: 1

;; OPT PSEUDOSECTION:
; EDNS: version: 0, flags:; udp: 1280
;; QUESTION SECTION:
;_redirect.301.systems.		IN	TXT

;; ANSWER SECTION:
_redirect.301.systems.	3600	IN	TXT	"https://github.com/pingiun/Redirector"

;; Query time: 1104 msec
;; SERVER: 192.168.1.1#53(192.168.1.1)
;; WHEN: Wed Apr 11 16:24:26 CEST 2018
;; MSG SIZE  rcvd: 100
```

You should now be redirected when you visit yourdomain.tld.

## Running
If you want to host the rust service yourself you can take these steps:
1. Clone this repository (`git clone https://github.com/pingiun/Redirector`)
2. Build the project with `cargo build --release` or with docker: `docker build -t redirector .`
3. When running the program, supply the `LISTEN_ADDR` environment variable, for example:
```terminal
LISTEN_ADDR=localhost:8080 RUST_LOG=main=info target/release/redirector
```
or in docker (example uses the docker hub version):
```terminal
docker run -e LISTEN_ADDR=0.0.0.0:80 -p 80:80 -d pingiun/redirector
```

## Deploying on kubernetes
This project has supplied a kubernetes deployment to run redirector with 3 replicas. You can change the deploy.yaml file to your needs and create the deployment with `kubectl create -f deploy.yaml`.
