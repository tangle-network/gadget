FROM rust:latest

WORKDIR /usr/src/server
COPY . .
RUN apt-get update && apt-get install -y protobuf-compiler
RUN cargo build --release --example zknode
RUN cargo run --release --example zk-test-client -- --circuit-id 1 --job-id 1 --wasm ./fixtures/sha256/circom.wasm --r1cs ./fixtures/sha256/circom.r1cs --input ./fixtures/sha256/input.json --public-inputs ./fixtures/sha256/public_inputs.json --output-dir ./target/zk --generate-proving-key
CMD RUST_LOG=debug cargo run --release --example zknode -- --n 8 --king-ip $BIND_ADDRESS --public-identity-der certs/0/cert.der --watch-dir target/zk --name king --i 0 --private-identity-der certs/0/key.der