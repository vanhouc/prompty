FROM rust:latest as builder

WORKDIR /usr/src/prompty
COPY . .
# Will build and cache the binary and dependent crates in release mode
RUN --mount=type=cache,target=/usr/local/cargo,from=rust:latest,source=/usr/local/cargo \
    --mount=type=cache,target=target \
    cargo build --release && mv ./target/release/prompty ./prompty

FROM debian:bullseye-slim

COPY --from=builder /usr/src/prompty/prompty /usr/local/bin/prompty
CMD ["prompty"]