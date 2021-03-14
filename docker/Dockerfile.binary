FROM clux/muslrust as cargo-build
WORKDIR /usr/src/ido-services

# Copy and Build Code
COPY . .
RUN cargo build --target x86_64-unknown-linux-musl --release

# Extract Binary
FROM alpine:latest

# Handle signal handlers properly
RUN apk add --no-cache tini
COPY --from=cargo-build /usr/src/ido-services/target/x86_64-unknown-linux-musl/release/orderbook /usr/local/bin/orderbook

CMD echo "Specify binary - either solver or orderbook"
ENTRYPOINT ["/sbin/tini", "--"]