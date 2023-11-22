FROM rust:latest

WORKDIR /usr/src/client
COPY . .
RUN apt-get update && apt-get install -y protobuf-compiler netcat-traditional
RUN cargo build --release --example zknode
RUN cargo run --release --example zk-test-client -- --circuit-id 1 --job-id 1 --wasm ./fixtures/sha256/circom.wasm --r1cs ./fixtures/sha256/circom.r1cs --input ./fixtures/sha256/input.json --public-inputs ./fixtures/sha256/public_inputs.json --output-dir ./target/zk --generate-proving-key
ENTRYPOINT ["./docker/client/client.sh"]