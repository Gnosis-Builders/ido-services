FROM clux/muslrust as cargo-build
WORKDIR /usr/src/app

# Copy and Build Code
COPY . /usr/src/app
RUN cargo build --target x86_64-unknown-linux-musl --release

# Extract Binary
FROM alpine:latest

# Handle signal handlers properly
RUN apk add --no-cache tini
COPY --from=cargo-build /usr/src/app/target/x86_64-unknown-linux-musl/release/orderbook /usr/local/bin/orderbook
EXPOSE 80
EXPOSE 8080

CMD echo "Specify binary - either solver or orderbook"
ENTRYPOINT ["/sbin/tini", "--"]
