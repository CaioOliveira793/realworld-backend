ARG RUST_VERSION=1.64

FROM docker.io/library/rust:${RUST_VERSION}-slim-bullseye as development

RUN echo "Dir::Cache \"\";\nDir::Cache::archives \"\";" | tee /etc/apt/apt.conf.d/00_disable-cache-directories && \
	apt update --quiet && \
	apt install --quiet -y git

RUN useradd conduit --create-home --home /home/conduit --user-group
USER conduit:conduit

WORKDIR /home/conduit/api

COPY --chown=conduit:conduit Cargo.toml Cargo.lock .

# NOTE: work around in slow `cargo fetch --locked`. https://github.com/rust-lang/cargo/issues/9177
RUN mkdir /home/conduit/.cargo && echo "[net]\ngit-fetch-with-cli = true" > /home/conduit/.cargo/config.toml
RUN cargo fetch --locked

COPY --chown=conduit:conduit . .

RUN cargo build --profile dev

CMD cargo run --profile dev
