FROM rust:1.66

WORKDIR /usr/src/prompty
COPY . .

RUN cargo install --path .

CMD ["prompty"]