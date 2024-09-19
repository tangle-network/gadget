## ECDSA Threshold MPC Blueprint

_Note: This Blueprint is under active development._

A Blueprint for running a ECDSA threshold MPC protocol on the Tangle Network.

## Building the Blueprint

- To build the blueprint, just run the following command:

```bash
cargo build -p ecdsa-threshold-mpc
```

- and to build the gadget that uses the blueprint, run the following command:

```bash
cargo build -p ecdsa-threshold-mpc --features=gadget
```
