FROM scratch
COPY target/x86_64-unknown-linux-musl/release/multipool-node /service
CMD ["/service"]
