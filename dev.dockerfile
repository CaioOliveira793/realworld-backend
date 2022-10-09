ARG RUST_VERSION=1.64

FROM docker.io/library/rust:${RUST_VERSION}-slim-bullseye as development

WORKDIR /usr/src/conduit_api

COPY Cargo.toml Cargo.lock .

RUN cargo fetch --locked

COPY . .

RUN cargo build --profile dev

CMD cargo run --profile dev
