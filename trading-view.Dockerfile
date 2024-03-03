FROM scratch
COPY target/x86_64-unknown-linux-musl/release/trading-view-api /service
CMD ["/service"]
