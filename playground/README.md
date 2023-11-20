## How to run this playground

1. Generate the Local Certificates:
```sh
./playground/generate_certs.sh
```
2. Generate the shares by running the client:
```sh
cargo run --release --bin client -- --circuit-id 1 --job-id 1 --wasm ./fixtures/sha256/circom.wasm --r1cs ./fixtures/sha256/circom.r1cs --input ./fixtures/sha256/input.json --public-inputs ./fixtures/sha256/public_inputs.json --output-dir ./target/zk --generate-proving-key
```
You can change the `--circuit-id` and `--job-id` to any number you want. The `--output-dir` is where the shares will be generated. You can change the `--wasm`, `--r1cs`, `--input` and `--public-inputs` to any circuit you want to run.
Note: `--generate-proving-key` will generate the proving key. If you already have the proving key, you can omit this flag.

3. Running the zknode(s):
You will need 8 terminals to run the zknode(s). In each terminal, run:

**King**

```sh
RUST_LOG=debug cargo run --release --bin zknode -- --king-ip 127.0.0.1:5555 \
    --certs certs/0/cert.der \
    --certs certs/1/cert.der \
    --certs certs/2/cert.der \
    --certs certs/3/cert.der \
    --certs certs/4/cert.der \
    --certs certs/5/cert.der \
    --certs certs/6/cert.der \
    --certs certs/7/cert.der \
    --watch-dir target/zk \
    --name king --i 0 \
    --private-identity-der certs/0/key.der
```

**Node 1**

```sh
RUST_LOG=debug cargo run --release --bin zknode -- --king-ip 127.0.0.1:5555 \
    --certs certs/0/cert.der \
    --certs certs/1/cert.der \
    --certs certs/2/cert.der \
    --certs certs/3/cert.der \
    --certs certs/4/cert.der \
    --certs certs/5/cert.der \
    --certs certs/6/cert.der \
    --certs certs/7/cert.der \
    --watch-dir target/zk \
    --name node1 --i 1 \
    --private-identity-der certs/1/key.der
```

**Node 2**
```sh
RUST_LOG=debug cargo run --release --bin zknode -- --king-ip 127.0.0.1:5555 \
    --certs certs/0/cert.der \
    --certs certs/1/cert.der \
    --certs certs/2/cert.der \
    --certs certs/3/cert.der \
    --certs certs/4/cert.der \
    --certs certs/5/cert.der \
    --certs certs/6/cert.der \
    --certs certs/7/cert.der \
    --watch-dir target/zk \
    --name node2 --i 2 \
    --private-identity-der certs/2/key.der
```

**Node 3**
```sh
RUST_LOG=debug cargo run --release --bin zknode -- --king-ip 127.0.0.1:5555 \
    --certs certs/0/cert.der \
    --certs certs/1/cert.der \
    --certs certs/2/cert.der \
    --certs certs/3/cert.der \
    --certs certs/4/cert.der \
    --certs certs/5/cert.der \
    --certs certs/6/cert.der \
    --certs certs/7/cert.der \
    --watch-dir target/zk \
    --name node3 --i 3 \
    --private-identity-der certs/3/key.der
```


**Node 4**
```sh
RUST_LOG=debug cargo run --release --bin zknode -- --king-ip 127.0.0.1:5555 \
    --certs certs/0/cert.der \
    --certs certs/1/cert.der \
    --certs certs/2/cert.der \
    --certs certs/3/cert.der \
    --certs certs/4/cert.der \
    --certs certs/5/cert.der \
    --certs certs/6/cert.der \
    --certs certs/7/cert.der \
    --watch-dir target/zk \
    --name node4 --i 4 \
    --private-identity-der certs/4/key.der
```

**Node 5**
```sh
RUST_LOG=debug cargo run --release --bin zknode -- --king-ip 127.0.0.1:5555 \
    --certs certs/0/cert.der \
    --certs certs/1/cert.der \
    --certs certs/2/cert.der \
    --certs certs/3/cert.der \
    --certs certs/4/cert.der \
    --certs certs/5/cert.der \
    --certs certs/6/cert.der \
    --certs certs/7/cert.der \
    --watch-dir target/zk \
    --name node5 --i 5 \
    --private-identity-der certs/5/key.der
```

**Node 6**
```sh
RUST_LOG=debug cargo run --release --bin zknode -- --king-ip 127.0.0.1:5555 \
    --certs certs/0/cert.der \
    --certs certs/1/cert.der \
    --certs certs/2/cert.der \
    --certs certs/3/cert.der \
    --certs certs/4/cert.der \
    --certs certs/5/cert.der \
    --certs certs/6/cert.der \
    --certs certs/7/cert.der \
    --watch-dir target/zk \
    --name node6 --i 6 \
    --private-identity-der certs/6/key.der
```

**Node 7**
```sh
RUST_LOG=debug cargo run --release --bin zknode -- --king-ip 127.0.0.1:5555 \
    --certs certs/0/cert.der \
    --certs certs/1/cert.der \
    --certs certs/2/cert.der \
    --certs certs/3/cert.der \
    --certs certs/4/cert.der \
    --certs certs/5/cert.der \
    --certs certs/6/cert.der \
    --certs certs/7/cert.der \
    --watch-dir target/zk \
    --name node7 --i 7 \
    --private-identity-der certs/7/key.der
```

4. Wait till the king finishes and outputs the final result.


