FROM rust:nightly-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p udd

FROM debian:stable-slim
WORKDIR /app
RUN useradd -u 10001 -r -s /usr/sbin/nologin udd
RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y krb5-admin-server krb5-user && \
	rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/udd /usr/local/bin/udd
COPY config /app/config
COPY deploy/certs /app/deploy/certs
USER udd
EXPOSE 8443
ENTRYPOINT ["/usr/local/bin/udd"]
