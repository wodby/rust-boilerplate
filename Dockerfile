ARG WODBY_BASE_IMAGE
FROM ${WODBY_BASE_IMAGE} AS builder

ARG COPY_FROM
# Download the dependencies in a layer of their own, so a change to the code alone reuses it.
# Cargo needs a target to read the manifest; the placeholder is replaced by the real code below.
COPY --chown=wodby:wodby ${COPY_FROM}/Cargo.toml ${COPY_FROM}/Cargo.lock /usr/src/app/
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo fetch --locked

COPY --chown=wodby:wodby ${COPY_FROM} /usr/src/app
RUN cargo build --release --locked && install -m 0755 target/release/app /home/wodby/.cargo/bin/app

FROM ${WODBY_BASE_IMAGE}
COPY --from=builder --chown=wodby:wodby /home/wodby/.cargo/bin/app /home/wodby/.cargo/bin/app

CMD ["app"]
