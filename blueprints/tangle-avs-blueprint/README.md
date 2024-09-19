## Tangle AVS Blueprint

_Note: This Blueprint is under active development._

A Blueprint for an EigenLayer AVS that runs a Tangle Validator.

## Building the Blueprint

- To build the blueprint, just run the following command:

```bash
cargo build -p tangle-avs-blueprint
```

- and to build the gadget that uses the blueprint, run the following command:

```bash
cargo build -p tangle-avs-blueprint --features=gadget
```
