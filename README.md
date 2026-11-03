# Rust starter for Wodby

A production-oriented HTTP application for the [Wodby Rust service](https://github.com/wodby/service-rust) and [Rust stack](https://github.com/wodby/stack-rust).

It uses [axum](https://github.com/tokio-rs/axum) on Tokio and demonstrates:

- typed routes with a not-found fallback and method-not-allowed responses
- HTML and static assets compiled into the binary
- JSON, health, and status responses
- request timeouts and graceful SIGTERM shutdown
- structured request logging, tests, rustfmt, clippy, and Wodby CI

## Local development

```shell
cargo test
cargo run
```

Open <http://localhost:8080>. Useful endpoints are:

- `/` — the embedded HTML landing page
- `/assets/styles.css` — an embedded static resource
- `/api/status` — a JSON response
- `/healthz` — the deployment health endpoint

Set `PORT` and `HOST` to listen elsewhere, and `RUST_LOG` (for example `debug`)
to change the log level.

## Project structure

`src/main.rs` contains the server lifecycle and `src/routes.rs` the handlers
and their tests. Files under `web/` are compiled into the binary with
`include_str!`, so the runtime image needs only the application binary.
`build.rs` records the compiler version shown on the home page.

The package is named `app`, and the Dockerfile installs the release build of
that binary. Rename the package and update the Dockerfile together.

The liveness endpoint intentionally avoids optional services. PostgreSQL,
Valkey, and SMTP links can be enabled later through Wodby's documented
environment variables.

## Development workspace

In a Wodby development workspace the checkout is compiled with `cargo build`
and run, then compiled again and restarted whenever a file changes. Commit
`Cargo.lock`: the workspace and the production build use it as is.
