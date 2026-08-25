# nse_fo_decoder

A reverse-engineered decoder for NSE F&O (NNF) broadcast feed messages.

Reads raw datagram bytes and decodes the bundled/compressed messages inside
into typed [`Message`] values (also serializable to JSON via `serde`).

## Status

This is a reverse-engineered wire format, not derived from an official spec.
Large parts of the layout are confirmed only against a small number of
captured packets. Every module in this crate documents its own confidence
level (HIGH / MEDIUM / UNKNOWN / UNVALIDATED) inline — read those before
trusting a field in production.

Supported message types: 7208 (MBP), 17208 (Enhanced MBP), 7202 (Ticker /
Market Index / OI), 7305 (Security price band update), 7220 (Trade execution
range). 7200 (legacy combined MBO+MBP) is deliberately not decoded — see the
comment on `decode::dispatch`.

## Usage

```rust
let raw_datagram: &[u8] = /* ... */;
let messages = nse_fo_decoder::decode_packet(raw_datagram)?;
for msg in &messages {
    println!("{}", serde_json::to_string(msg)?);
}
```

## LZO1Z dependency

Some messages arrive LZO1Z-compressed. Decompression is done via runtime
dynamic loading (`libloading`) of `liblzo2`, which must be present on the
host and resolvable via the normal OS library search path. Most Rust LZO
crates implement LZO1X, not LZO1Z, and will not work here regardless of
platform.

Platform support: **Windows is verified** (`liblzo2-2.dll`, confirmed loadable
and round-tripped through the real library in `cargo test`). Linux/macOS try
conventional `liblzo2` SONAMEs (`liblzo2.so.2`, `liblzo2.dylib`, etc. -- see
`src/lzo.rs`) but this is unvalidated guesswork, not tested on those
platforms. If it fails to load there, check what your distro's package
actually installs and adjust the candidate list.

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or
  http://opensource.org/licenses/MIT)

at your option.
