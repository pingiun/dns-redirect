FROM rust:1.25.0

WORKDIR /usr/src/redirector
COPY . .

RUN cargo install

ENV RUST_LOG main=info

CMD ["redirector"]