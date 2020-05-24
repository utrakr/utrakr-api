FROM rust:1.43.1 as build

# app name
ENV app=utrakr-api

# fetch dependencies
WORKDIR /tmp
RUN USER=root cargo new ${app}
WORKDIR /tmp/${app}
COPY Cargo.toml Cargo.lock /tmp/${app}/
RUN cargo fetch

# compile dependencies
RUN mkdir -p /tmp/${app}/src/\
 && echo 'fn main() {}' > /tmp/${app}/src/main.rs
RUN cargo build --release

# copy source and rebuild
COPY src/ /tmp/${app}/src/
RUN cargo build --release

# copy binary into smaller image with same base as rust
FROM debian:buster-slim
COPY --from=build /tmp/utrakr-api/target/release/utrakr-api /

ENV RUST_LOG="utrakr_api=debug,info"
ENV RUST_BACKTRACE=full
CMD ["/utrakr-api"]
