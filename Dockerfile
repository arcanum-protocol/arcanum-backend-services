ARG BIN
FROM scratch
COPY target/x86_64-unknown-linux-musl/release/${BIN} /service
CMD ["/service"]
