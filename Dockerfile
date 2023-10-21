FROM rust:alpine as builder
ENV RUSTFLAGS="-C target-feature=-crt-static"

# install musl-dev to build static binaries
RUN apk add --no-cache musl-dev

# copy the source code
WORKDIR /app
COPY ./ /app

# do a release build
RUN cargo build --release
RUN strip target/release/tars

# use a plain alpine image, the alpine version needs to match the builder
FROM alpine:latest as runtime
RUN apk add --no-cache libgcc

# create a non-root user
RUN addgroup -g 1500 tars && \
    adduser -H -D -u 1500 -G tars tars

# copy the binary from the builder
WORKDIR /app
COPY --from=builder /app/target/release/tars .
RUN chown -R tars:tars /app && chmod +x tars

USER tars

ENTRYPOINT ["./tars"]