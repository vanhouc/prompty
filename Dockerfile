FROM rust:1.66 as builder
WORKDIR /usr/src/prompty
COPY . .
RUN cargo install --path .

FROM debian:buster
COPY --from=builder /usr/local/cargo/bin/prompty /usr/local/bin/prompty
CMD ["prompty"]