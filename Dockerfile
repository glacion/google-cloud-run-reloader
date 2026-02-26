FROM gcr.io/distroless/static-debian12:nonroot
ENV RUST_LOG=info
COPY ./reloder /usr/local/bin/reloder
ENTRYPOINT ["/usr/local/bin/reloder"]
