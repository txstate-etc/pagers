FROM rust:1.41.0 AS build
WORKDIR /usr/src

# Download the target for static linking.
RUN apt-get update \
  && DEBIAN_FRONTEND=noninteractive apt-get -y install ca-certificates musl-dev musl-tools \
  #libssl-dev make automake \
  && update-ca-certificates \
  && rustup target add x86_64-unknown-linux-musl

# Create a dummy project and build the app's dependencies.
# If the Cargo.toml or Cargo.lock files have not changed,
# docker build cache will be used to skip these slow steps.
RUN USER=root cargo new pagers
WORKDIR /usr/src/pagers
COPY Cargo.* ./
RUN cargo build --release

# Copy the source and build the application and health checker
COPY src ./src
RUN export OPENSSL_DIR=/usr \
  && cargo install --target x86_64-unknown-linux-musl --path . \
  && mkdir -p /rootfs/var/lib/pagers /rootfs/etc/ssl /rootfs/bin/ \
  && chown -R 48.48 /rootfs/var/lib/pagers/ \
  && cp -r /etc/ssl/certs /rootfs/etc/ssl/ \
  && cp /usr/local/cargo/bin/pagers /rootfs/bin/

FROM scratch
COPY --from=build /rootfs/  /
USER 48:48
WORKDIR /var/lib/pagers/
CMD ["/bin/pagers"]
