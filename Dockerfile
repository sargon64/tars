FROM rust:alpine

COPY . .

RUN cargo build --release

EXPOSE 8080
CMD ["./target/release/ta-relay-rs"]