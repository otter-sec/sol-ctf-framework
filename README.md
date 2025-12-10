# Solana CTF Framework

## Usage

To get started, just add the following line to the `dependencies` section in your `Cargo.toml`:
```toml
[dependencies]
sol-ctf-framework = { git = "https://github.com/otter-sec/sol-ctf-framework.git" }
```

You might need to manually set base64ct to 1.6.0 to be able to use cargo-build-sbf with edition 2021

```bash
cargo update --precise 1.6.0 -p base64ct@1.8.0
```

## What is this for?
This crate is meant to be used to create an environment for capture the flag players to solve challenges related to solana.

