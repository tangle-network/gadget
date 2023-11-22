#!/bin/sh
set -e
trap "exit" INT TERM
trap "kill 0" EXIT

# Step 0: cd to git working dir
cd "$(git rev-parse --show-toplevel)"

# Step 1: check to see if arg "--create-certs" was passed
if [ "$1" = "--create-certs" ]; then
    # Step 2: if so, invoke the generate_certs.sh script
    echo "Creating certs..."
    ./playground/generate_certs.sh
fi

# Step 2: Run the client unless --no-client was passed as the second or first parameter
if [ "$1" != "--no-client" ] && [ "$2" != "--no-client" ]; then
    cargo run --release --example zk-test-client -- --circuit-id 1 --job-id 1 --wasm ./fixtures/sha256/circom.wasm --r1cs ./fixtures/sha256/circom.r1cs --input ./fixtures/sha256/input.json --public-inputs ./fixtures/sha256/public_inputs.json --output-dir ./target/zk --generate-proving-key
fi

# Step 3: for x in 0..=7, run a command
PROCS=()
SERVER_IP=127.0.0.1
SERVER_PORT=5555
n=8
for i in {0..7}
  do
    # if i is zero, run the king
    if [ "$i" -eq "0" ]; then
        # Step 4: Run the king
        RUST_LOG=debug cargo run --release --example zknode -- --n $n --king-ip $SERVER_IP:$SERVER_PORT --public-identity-der certs/$i/cert.der --watch-dir target/zk --name king --i $i --private-identity-der certs/0/key.der &
        pid=$!
        PROCS[$i]=$pid

        # But first, wait for the king to start
        while ! nc -z -w 5 $SERVER_IP $SERVER_PORT; do
          echo "Waiting for server $SERVER_IP:$SERVER_PORT ... "
          sleep 0.5
        done
    else
        # Step 4: Run the client
        RUST_LOG=debug cargo run --release --example zknode -- --n $n --king-ip $SERVER_IP:$SERVER_PORT --public-identity-der certs/$i/cert.der --client-only-king-public-identity-der certs/0/cert.der --watch-dir target/zk --name node$i --i $i --private-identity-der certs/$i/key.der &
        pid=$!
        PROCS[$i]=$pid
        sleep 0.2
    fi
  done

# Step 5: wait for all processes to finish
for pid in ${PROCS[@]}; do
    wait $pid
done