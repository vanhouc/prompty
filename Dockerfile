FROM rust:1.66 as builder

WORKDIR /usr/src/prompty
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
# If system packages are needed uncomment the following line and add them in place of "extra-runtime-dependencies"
# RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/prompty /usr/local/bin/prompty

CMD ["prompty"]