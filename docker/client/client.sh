#!/usr/bin/env bash
set -e
trap "exit" INT TERM
trap "kill 0" EXIT

echo "Server addr: $SERVER_IP:$SERVER_PORT"

# Retry loop
while ! nc -z -w 5 $SERVER_IP $SERVER_PORT; do
  echo "Waiting for server $SERVER_IP:$SERVER_PORT ... "
  sleep 0.5
done

PROCS=()
n=8
for i in {1..7}
  do
    RUST_LOG=debug cargo run --release --example zknode -- --n $n --king-ip $SERVER_IP:$SERVER_PORT --public-identity-der certs/$i/cert.der --client-only-king-public-identity-der certs/0/cert.der --watch-dir target/zk --name node$i --i $i --private-identity-der certs/$i/key.der &
    pid=$!
    PROCS[$i]=$pid
  done

# Step 5: wait for all processes to finish
for pid in ${PROCS[@]}; do
    wait $pid
done