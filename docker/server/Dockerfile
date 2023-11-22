FROM rust:latest

WORKDIR /usr/src/server
COPY . .
RUN apt-get update && apt-get install -y protobuf-compiler
RUN sleep 15 # give time for the clients to get ahead in compiling
CMD RUST_LOG=debug cargo run --release --example zknode -- --n 8 --king-ip $BIND_ADDRESS --public-identity-der certs/0/cert.der --watch-dir target/zk --name king --i 0 --private-identity-der certs/0/key.der