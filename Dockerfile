FROM rust:1.91-alpine AS build

WORKDIR /usr/src/reloader
ADD Cargo.lock Cargo.toml ./
ADD src ./src
RUN --mount=type=cache,target=/usr/src/reloader/target --mount=type=cache,target=/root/.cargo cargo build --release
RUN --mount=type=cache,target=/usr/src/reloader/target install -m 755 target/release/reloader /usr/local/cargo/bin/reloader

FROM gcr.io/distroless/static-debian12:nonroot
ENV RUST_LOG=info
COPY --from=build /usr/local/cargo/bin/reloader /usr/local/bin/reloader
CMD ["/usr/local/bin/reloader"]
