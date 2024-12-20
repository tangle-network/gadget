## Incredible Squaring Blueprint for Symbiotic

A simple blueprint that only has one job that takes **x** and returns **x<sup>2</sup>**.

## Prerequisites

Before you begin, ensure you have the following installed:

- [Anvil](https://book.getfoundry.sh/anvil/)
- [Docker](https://www.docker.com/get-started)

## Installation

1. Clone this repository:
   ```bash
   git clone https://github.com/tangle-network/gadget.git
   cd gadget
   ```
   
2. Install Anvil:
   ```bash
   curl -L https://foundry.paradigm.xyz | bash
   foundryup
   ```

## Building the Blueprint

- To build the blueprint, just run the following command:

```bash
cargo build -p incredible-squaring-blueprint-symbiotic
```

## Running the AVS on a Testnet

- We have a test for running this AVS Blueprint on a local Anvil Testnet. You can run the test with the following:

```bash
RUST_LOG=gadget=trace cargo test --package incredible-squaring-blueprint-symbiotic test_symbiotic_incredible_squaring_blueprint -- --nocapture
```