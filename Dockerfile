FROM rust:1.91-slim-bullseye AS build

WORKDIR /usr/src/reloader
ADD Cargo.lock Cargo.toml ./
ADD src ./src
RUN --mount=type=cache,target=/usr/src/reloader/target cargo build --release
RUN --mount=type=cache,target=/usr/src/reloader/target install --compare --mode=755 target/release/reloader /usr/local/cargo/bin/reloader

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /usr/local/cargo/bin/reloader /usr/local/bin/reloader
CMD ["/usr/local/bin/reloader"]

