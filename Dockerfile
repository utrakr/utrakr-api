FROM rust:1.44 as build

# app
ENV app=utrakr-api

# dependencies
WORKDIR /tmp/${app}
COPY Cargo.toml Cargo.lock ./

# compile dependencies
RUN set -x\
 && mkdir -p src\
 && echo "fn main() {println!(\"broken\")}" > src/main.rs\
 && cargo build --release

# copy source and rebuild
COPY src/ src/
RUN set -x\
 && find target/release -type f -name "$(echo "${app}" | tr '-' '_')*" -exec touch -t 200001010000 {} +\
 && cargo build --release

# check
RUN ["/bin/bash", "-c", "set -x && /tmp/${app}/target/release/${app} --version | grep ${app}"]

# copy binary into smaller image with same base as rust
FROM debian:buster-slim
COPY --from=build /tmp/utrakr-api/target/release/utrakr-api /

ENV RUST_LOG="utrakr_api=debug,info"
ENV RUST_BACKTRACE=full
CMD ["/utrakr-api"]
