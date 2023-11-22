# A simple script that invokes a simple binary to generate a self-signed
# certificate for use in the playground.

for i in {0..7}
  do
      cargo run --example certgen -- --i $i
  done
echo "Done generating certs"
