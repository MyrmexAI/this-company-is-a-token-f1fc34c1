# AntPad launchpad program

Pinocchio program (no Anchor). Tokens are created and traded on pump.fun; this program only
keeps a registry of launches made through AntPad and charges a fixed SOL platform fee that goes
straight from the creator to the owner's treasury wallet. The program never holds user funds.

Build and test: `cargo build-sbf && cargo test` (tests load `target/deploy/antpad_launchpad.so` into LiteSVM).

`declare_id!` in `programs/launchpad/src/lib.rs` is a placeholder; the platform sets the real id before deploy.

## Accounts (PDAs)

- Config: seeds `["config"]`, 74 bytes: disc `1` | owner(32) | treasury(32) | fee_lamports(u64 LE) | bump.
- Launch: seeds `["launch", mint]`, 74 bytes: disc `2` | creator(32) | mint(32) | created_at(i64 LE, unix) | bump.

## Instructions (first data byte = discriminator)

| # | Name | Accounts | Data after discriminator |
|---|------|----------|--------------------------|
| 0 | `init` | payer (s,w), owner (s), config (w), system program | treasury(32), fee_lamports(u64 LE) |
| 1 | `set_fee` | owner (s), config (w) | fee_lamports(u64 LE) |
| 2 | `set_treasury` | owner (s), config (w) | treasury(32) |
| 3 | `register_launch` | creator (s,w), config, treasury (w), launch PDA (w), mint, system program | none |

Rules: fee is capped at 1 SOL (1_000_000_000 lamports); `init` works once; only the owner (signer) can
call `set_fee` / `set_treasury`; a zero treasury is rejected; `register_launch` requires an initialized
SPL Token or Token-2022 mint, the treasury account must equal the configured treasury, and a mint can be
registered once. A fee of 0 skips the transfer.

## Notes for operators

- `init` takes the owner as a signer, but anyone can call it first. Run `init` in the same step as the
  deploy, and check that the config owner is the intended wallet.
- The treasury must be a system wallet. If it does not exist yet, the fee must be at least the
  rent-exempt minimum for an empty account (about 0.00089 SOL), otherwise the transfer fails.
- `register_launch` does not prove the creator really created the pump.fun mint (the mint authority is
  gone after creation). Anyone can register an existing mint by paying the fee, so the first
  registration wins. The website should register right after creating the token.
