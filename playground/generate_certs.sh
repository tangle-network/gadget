#!/bin/sh
# A simple script that invokes a simple binary to generate a self-signed
# certificate for use in the playground.
n=8
for i in $(seq 0 $(($n - 1)))
  do
      cargo run --release --example certgen -- --i $i
  done
echo "Done generating certs"
