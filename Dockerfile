FROM scratch
ARG BIN
COPY target/x86_64-unknown-linux-musl/release/${BIN} /service
COPY /logos /logos
# COPY /etl/config.yaml /config.yaml
CMD ["/service"]
