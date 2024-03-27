FROM scratch
ARG BIN
COPY target/x86_64-unknown-linux-musl/release/${BIN} /service
CMD ["/service"]
