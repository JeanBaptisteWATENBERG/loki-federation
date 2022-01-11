FROM rust:1.57 as builder

WORKDIR ./loki-federation

ADD . ./
ENV RUSTC_BOOTSTRAP=1

RUN rustup toolchain install stable-x86_64-unknown-linux-gnu
RUN rustup default stable-x86_64-unknown-linux-gnu
RUN rustup component add rustfmt
RUN cargo build --release

FROM alpine:latest

RUN mkdir -p /run/app

COPY --from=builder /loki-federation/target/release/loki-federation /run/app/loki-federation

WORKDIR /run/app

CMD ["./loki-federation"]
