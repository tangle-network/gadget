# A simple script that invokes a simple binary to generate a self-signed
# certificate for use in the playground.

# 1. Build the binary
cargo build --example certgen
# 2. Run the binary
# Generate 8 certs from 0 to 7
for i in {0..7}
  do
      ./target/debug/certgen --i $i
  done
echo "Done generating certs"
