ARG RUST_VERSION=1.64

FROM docker.io/library/rust:${RUST_VERSION}-slim-bullseye as development

RUN useradd conduit --create-home --home /home/conduit --user-group
USER conduit:conduit

WORKDIR /home/conduit/api

COPY --chown=conduit:conduit Cargo.toml Cargo.lock .

RUN cargo fetch --locked

COPY --chown=conduit:conduit . .

RUN cargo build --profile dev

CMD cargo run --profile dev
