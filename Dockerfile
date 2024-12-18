# # Start with a rust alpine image
# FROM rust:1-alpine3.19
# # This is important, see https://github.com/rust-lang/docker-rust/issues/85
# ENV RUSTFLAGS="-C target-feature=-crt-static"
# # if needed, add additional dependencies here
# RUN apk add --no-cache musl-dev
# # set the workdir and copy the source into it
# WORKDIR /app
# COPY ./ /app
# # do a release build
# RUN cargo build --release

# # use a plain alpine image, the alpine version needs to match the builder
# FROM alpine:3.19
# # if needed, install additional dependencies here
# RUN apk add --no-cache libgcc
# # copy the binary into the final image
# COPY --from=0 /app/target/release/ack-relay .
# # set the binary as entrypoint
# ENTRYPOINT ["/ack-relay"]

# EXPOSE 3000

FROM clux/muslrust:stable AS chef
USER root
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Notice that we are specifying the --target flag!
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl --bin ack-relay

FROM alpine AS runtime
# RUN addgroup -S myuser && adduser -S myuser -G myuser
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/ack-relay /usr/local/bin/ack-relay
# USER myuser
CMD ["/usr/local/bin/ack-relay"]