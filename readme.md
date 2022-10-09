# RealWorld rust backend implementation

A rust implementation of the [RealWorld](https://github.com/gothinkster/realworld) [Conduit](https://demo.realworld.io/#/) example app.

## Stack

- [Salvo](https://salvo.rs/), the simplest rust web framework
- [Tokio](https://tokio.rs/) asynchronous runtime
- [Tokio postgres](https://docs.rs/tokio-postgres/latest/tokio_postgres/) client
- [Sea query](https://docs.rs/sea-query/latest/sea_query/) builder

## Run

To run in develpment you need a container engine available, e.g. docker or podman

Using the compose file you can run:

```sh
APP_ENV=development docker-compose up -d
```

## Test

The unit tests are located in the test modules through the codebase.

To run the unit tests use:

```sh
cargo test --lib
```

The e2e tests uses the same config as the development env, although the persisted data (database volumes) are isolated between environments.

Before run the tests start the app with:

```sh
APP_ENV=test docker-compose up -d
```

To run the e2e tests with the application started, run:

```sh
cargo test --tests
```

<!-- TODO: architecture -->
