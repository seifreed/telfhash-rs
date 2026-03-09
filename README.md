<p align="center">
  <img src="https://img.shields.io/badge/telfhash--rs-ELF%20Similarity%20Hashing-blue?style=for-the-badge" alt="telfhash-rs">
</p>

<h1 align="center">telfhash-rs</h1>

<p align="center">
  <strong>Rust 2024 implementation of Trend Micro telfhash for ELF similarity hashing</strong>
</p>

<p align="center">
  <a href="https://crates.io/crates/telfhash-rs"><img src="https://img.shields.io/crates/v/telfhash-rs?style=flat-square&logo=rust&logoColor=white" alt="Crates.io Version"></a>
  <a href="https://crates.io/crates/telfhash-rs"><img src="https://img.shields.io/badge/rust-2024-orange?style=flat-square&logo=rust&logoColor=white" alt="Rust Edition"></a>
  <a href="https://github.com/seifreed/telfhash-rs/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-green?style=flat-square" alt="License"></a>
  <a href="https://github.com/seifreed/telfhash-rs/actions"><img src="https://img.shields.io/github/actions/workflow/status/seifreed/telfhash-rs/ci.yml?style=flat-square&logo=github&label=CI" alt="CI Status"></a>
  <img src="https://img.shields.io/badge/tests-golden%20%2B%20integration-brightgreen?style=flat-square" alt="Tests">
</p>

<p align="center">
  <a href="https://github.com/seifreed/telfhash-rs/stargazers"><img src="https://img.shields.io/github/stars/seifreed/telfhash-rs?style=flat-square" alt="GitHub Stars"></a>
  <a href="https://github.com/seifreed/telfhash-rs/issues"><img src="https://img.shields.io/github/issues/seifreed/telfhash-rs?style=flat-square" alt="GitHub Issues"></a>
  <a href="https://github.com/seifreed/tlsh-rs"><img src="https://img.shields.io/badge/TLSH-tlsh--rs-yellow?style=flat-square" alt="tlsh-rs"></a>
</p>

---

## Overview

**telfhash-rs** is a Rust 2024 port of the original [`telfhash`](https://github.com/trendmicro/telfhash) project from Trend Micro. It computes ELF similarity hashes compatible with the original workflow and exposes them through both a CLI and a typed Rust library API.

This project exists so we can use `telfhash` natively from our Rust projects while keeping compatibility with the original algorithm and output expectations.

### Thanks

Special thanks to **Trend Micro** for the original `telfhash` work:

- Original project: <https://github.com/trendmicro/telfhash>

This repository ports that work to Rust and keeps the algorithm usable in native Rust codebases.

For TLSH generation and distance calculation, this project uses the Rust crate:

- `tlsh-rs`: <https://github.com/seifreed/tlsh-rs>

### Key Features

| Feature | Description |
|---------|-------------|
| **Algorithm Compatibility** | Preserves the original `telfhash` hashing behavior for ELF inputs |
| **Rust 2024** | Native Rust implementation designed for integration into Rust projects |
| **CLI + Library** | Use it as a command-line tool or from Rust code |
| **Multiple Output Formats** | Plain, TSV, JSON, and SARIF output |
| **Grouping Support** | Compatible legacy grouping and native `connected-components` grouping |
| **Structured Testing** | Golden tests, integration tests, architecture contract tests, and doctests |
| **Optional Logging** | `tracing`-based logging via the `logging` feature |

### What It Handles

```text
Input            ELF binaries and shared objects
Hashing          telfhash-compatible ELF similarity hashing
Fallback         Symbol-table extraction and disassembly-based call-destination recovery
Grouping         Legacy-compatible heuristic and native connected-components mode
Output           plain, tsv, json, sarif
Integration      CLI and typed Rust API
```

---

## Installation

### From Source

```bash
git clone https://github.com/seifreed/telfhash-rs.git
cd telfhash-rs
cargo build --release
```

### Development Setup

```bash
git clone https://github.com/seifreed/telfhash-rs.git
cd telfhash-rs
cargo test
cargo test --features logging
```

---

## Quick Start

```bash
# Hash ELF files
cargo run --bin telfhash -- tests/fixtures/bin/*

# Group similar files
cargo run --bin telfhash -- --group tests/fixtures/bin/*

# Emit SARIF
cargo run --bin telfhash -- -f sarif tests/fixtures/bin/*
```

---

## Usage

### Command Line Interface

```bash
# Basic hashing
telfhash sample.elf

# Hash multiple inputs
telfhash samples/*

# Group in compatible mode
telfhash --group samples/*

# Group in native connected-components mode
telfhash --group --group-mode connected-components samples/*

# JSON output
telfhash -f json samples/*

# SARIF output
telfhash -f sarif samples/*

# Recursive expansion
telfhash -r samples/**

# Debug mode
telfhash -d samples/example.elf
```

### Available Options

| Option | Description |
|--------|-------------|
| `-g, --group` | Group comparable hashes |
| `-t, --threshold` | Grouping threshold |
| `--group-mode` | `compatible` or `connected-components` |
| `-r, --recursive` | Expand paths recursively |
| `-o, --output` | Write output to file |
| `-f, --format` | Output format: `tsv`, `json`, `sarif` |
| `-d, --debug` | Emit debug diagnostics to stderr |

---

## Rust Library

### Basic Usage

```rust
use std::path::PathBuf;
use telfhash_rs::{GroupingMode, TelfhashEngine, TelfhashOptions};

let engine = TelfhashEngine::new();
let options = TelfhashOptions {
    grouping_mode: GroupingMode::Compatible,
    ..Default::default()
};

let results = engine
    .hash_paths(
        [PathBuf::from("tests/fixtures/bin/x86_64_dyn_stripped.so")],
        &options,
    )
    .unwrap();

for result in results {
    println!("{:?}", result.outcome);
}
```

### Group Existing Results

```rust
use std::path::PathBuf;
use telfhash_rs::{GroupingMode, TelfhashEngine, TelfhashOptions};

let engine = TelfhashEngine::new();
let options = TelfhashOptions {
    grouping_mode: GroupingMode::ConnectedComponents,
    threshold: 50,
    ..Default::default()
};

let results = engine
    .hash_paths(
        [
            PathBuf::from("tests/fixtures/bin/i386_dyn_stripped.so"),
            PathBuf::from("tests/fixtures/bin/x86_64_dyn_stripped.so"),
        ],
        &options,
    )
    .unwrap();

let grouped = engine.group(&results, &options).unwrap();
println!("{:?}", grouped.grouped);
```

---

## Output Formats

### Plain

```bash
telfhash samples/*
```

### TSV

```bash
telfhash -f tsv samples/*
```

### JSON

```bash
telfhash -f json samples/*
```

### SARIF

```bash
telfhash -f sarif samples/*
```

---

## Requirements

- Rust 2024 edition
- Cargo
- See [Cargo.toml](Cargo.toml) for dependency details

---

## Contributing

Contributions are welcome, but this project is strict about compatibility and architecture boundaries.

Before opening a PR:

1. Read the architecture and public API docs
2. Preserve compatibility unless the change explicitly intends to break it
3. Run:

```bash
cargo fmt
cargo test
cargo test --features logging
```

---

## License

This project is licensed under the Apache 2.0 License. See [LICENSE](LICENSE).

### Attribution

- Original algorithm and project: **Trend Micro** | <https://github.com/trendmicro/telfhash>
- Rust port and project maintenance: **Marc Rivero** | [@seifreed](https://github.com/seifreed)
- TLSH Rust crate used by this project: <https://github.com/seifreed/tlsh-rs>

---

<p align="center">
  <sub>Thanks to Trend Micro for the original telfhash work, which made this Rust port possible.</sub>
</p>
