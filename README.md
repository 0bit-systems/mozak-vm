![CI status](https://github.com/0xmozak/mozak-vm/actions/workflows/ci.yml/badge.svg)
![Unused dependencies status](https://github.com/0xmozak/mozak-vm/actions/workflows/unused-deps.yml/badge.svg)
![MacOS CI status](https://github.com/0xmozak/mozak-vm/actions/workflows/macos-ci.yml/badge.svg)

# Mozak Risc-V Virtual Machine

If you are unfamiliar with the Risc-V instruction set, please have a look at the [Risc-V instruction set reference](https://github.com/jameslzhu/riscv-card/blob/master/riscv-card.pdf).

# Build requirements

- [Rust toolchain](https://www.rust-lang.org/tools/install)

The Mozak VM is built in Rust, so the Rust toolchain is a pre-requisite.

```bash
cargo build
```

# Running test

To run all the tests in this repo, use:
```bash
cargo test
```

Selectively run tests using the following command:
```bash
cargo test --package <pkg> --lib -- <testname> --exact --nocapture
```

For example:
```bash
cargo test --package mozak-circuits --lib -- cross_table_lookup::tests::test_ctl --exact --nocapture
```

# Running

We have a rudimentary CLI.  You can run it via eg `cargo run -- -vvv run vm/tests/testdata/rv32ui-p-addi`.

Use `cargo run -- --help` to see what sub-commands are implemented.

# Update official Risc-V tests

- [Docker](https://www.docker.com/)

Updating the official Risc-V tests relies on Docker to install the RISC-V toolchain and build the ELF files necessary for our tests to run.

The Mozak VM implements the base RV32I instruction set with the M-extension,
so we are using rv32ui and rv32um ELF files from the [riscv-software-src/riscv-tests](https://github.com/riscv-software-src/riscv-tests) repo.

You can update the tests via `./update_testdata` in the root of the repository.

# Updating Rust toolchain

To update the Rust toolchain, change `rust-toolchain.toml`.
