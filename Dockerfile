FROM rust:1.66 as builder
WORKDIR /usr/src/prompty
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y openssl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/prompty /usr/local/bin/prompty
CMD ["prompty"]