## Incredible Squaring Blueprint for Eigenlayer

A simple AVS blueprint that only has one job - taking **x** and returning **x<sup>2</sup>**.

## Building the Blueprint

- To build the blueprint, just run the following command:

```bash
cargo build --release -p incredible-squaring-blueprint-eigenlayer
```

## Running the AVS on a Testnet

- We have a test for running this AVS Blueprint on a local Anvil Testnet. You can run the test with the following steps:

### Pre-requisites - Installation steps:

#### Anvil

```bash
curl -L https://foundry.paradigm.xyz | bash
foundryup
  ```
  
#### Docker

Install Docker, following the steps seen on the [official website](https://www.docker.com/get-started)

### Running the Test

The following command will run the test, automatically starting the testnet and the AVS.

```bash
RUST_LOG=gadget=trace cargo test --release --package blueprint-test-utils tests_standard::test_eigenlayer_incredible_squaring_blueprint -- --nocapture
```
