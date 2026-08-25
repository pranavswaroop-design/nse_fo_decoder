# nse_fo_decoder

A Rust decoder for NSE F&O (NNF) broadcast feed messages — a byte-for-byte
port of a reverse-engineered C++ reference implementation, cross-validated
against that reference's actual compiled output.

It reads raw UDP datagram bytes, walks the framing (outer prefix, bundled
sub-packets, optional LZO1Z decompression, message header), and decodes the
messages inside into typed, `serde`-serializable [`Message`] values.

## Status

This is a **reverse-engineered wire format**, not derived from an official
NSE specification. Large parts of the layout are confirmed only against a
small number of captured packets. Every module in this crate documents its
own confidence level (HIGH / MEDIUM / UNKNOWN / UNVALIDATED) inline as doc
comments — read those before trusting a field in production. See
[Known limitations](#known-limitations) below for the current list.

## Supported messages

| TransCode | Name | Module | Confidence |
|---|---|---|---|
| 7208 | MBP (Market by Price) | `decode::mbp` | HIGH |
| 17208 | Enhanced MBP | `decode::enhanced_mbp` | HIGH (token/levels) / UNKNOWN (OHLC etc.) / UNVALIDATED (multi-record) |
| 7202 | Ticker & Market Index / OI | `decode::ticker` | MEDIUM |
| 7305 | Security (price band) update | `decode::security_range` | MEDIUM |
| 7220 | Trade execution range | `decode::exec_range` | MEDIUM |
| 7200 | Legacy combined MBO+MBP | *not decoded* | LOW (deliberately unimplemented — see `decode::dispatch`) |

## Architecture

```
src/
├── lib.rs              public API: decode_packet(), Message, DecodeError
├── framing.rs           outer prefix -> BcastPackData -> BcastCmpPacket* walk,
│                         optional LZO1Z decompress, dispatch loop
├── header.rs             BCAST_HEADER field offsets (TransCode, BCSeqNo, MessageLength)
├── lzo.rs                LZO1Z decompression via runtime-loaded liblzo2
├── message.rs            output types (Message enum + one struct per message kind)
├── error.rs               DecodeError
├── decode/
│   ├── mod.rs              dispatch(trans_code, ...) -> routes to the modules below
│   ├── mbp.rs               7208
│   ├── enhanced_mbp.rs      17208
│   ├── ticker.rs            7202
│   ├── security_range.rs    7305
│   └── exec_range.rs        7220
└── bin/
    └── nse_fo_decoder.rs   CLI: hex-per-line on stdin -> JSON-array-per-line on stdout
```

`framing::decode_packet` is the single entry point: it owns the byte-level
walk and calls `decode::dispatch` once per message header it finds, which
routes to the matching `decode::*` module by `TransCode`. Each `decode::*`
module reads its fields directly from byte offsets via `byteorder` (no
`#[repr(packed)]` struct-overlay/pointer-cast tricks — see
[Design notes](#design-notes)).

## JSON output schemas

```jsonc
// 7208 / 17208 -> Message::Mbp
{
  "transaction_code": 7208, "msg_type": "MBP",
  "token": 0, "sequence": 0,
  "ltp": 0, "atp": 0, "close": 0, "ltq": 0, "ltt": 0,
  "open": 0, "high": 0, "low": 0,
  "tbq": 0, "tsq": 0, "total_traded_qty": 0,
  "entries": [
    { "md_entry_type": 0, "level": 1, "price": 0, "qty": 0, "orders": 0 }
    // 5 buy (md_entry_type 0, level 1-5) + 5 sell (md_entry_type 1, level 1-5)
  ]
}

// 7202 -> Message::Ticker (no "sequence" field)
{
  "transaction_code": 7202, "msg_type": "OI_TICKER",
  "token": 0, "market_type": 0,
  "fill_price": 0, "fill_volume": 0,
  "open_interest": 0, "day_high_oi": 0, "day_low_oi": 0
}

// 7305 -> Message::SecurityRange (no "sequence" field)
{
  "transaction_code": 7305, "msg_type": "SECURITY_RANGE",
  "token": 0, "low_price_range": 0, "high_price_range": 0
}

// 7220 -> Message::ExecRange
{
  "transaction_code": 7220, "msg_type": "EXEC_RANGE",
  "entries": [ { "token": 0, "high_exec_band": 0, "low_exec_band": 0 } ]
}
```

For 17208, `ltp`..`total_traded_qty` are always `0` — see
[Known limitations](#known-limitations).

## Usage

As a library:

```toml
[dependencies]
nse_fo_decoder = { path = "../path/to/nse_fo_decoder" }
```

```rust
let raw_datagram: &[u8] = /* one UDP payload's worth of bytes */;
let messages = nse_fo_decoder::decode_packet(raw_datagram)?;
for msg in &messages {
    println!("{}", serde_json::to_string(msg)?);
}
```

As the CLI binary — one hex-encoded datagram per line on stdin, one JSON
array per line on stdout:

```
$ echo <hex-encoded-datagram> | cargo run --bin nse_fo_decoder
[{"transaction_code":7305,"msg_type":"SECURITY_RANGE","token":12345,"low_price_range":100,"high_price_range":200}]
```

## Building and testing

```
cargo build              # library + CLI binary
cargo test                # unit tests, including the LZO round-trip self-test
cargo doc --open          # browse the API docs generated from doc comments
```

## LZO1Z dependency

Some messages arrive LZO1Z-compressed. Decompression is done via runtime
dynamic loading (`libloading`) of `liblzo2`, which must be present on the
host and resolvable via the normal OS library search path. Most Rust LZO
crates implement LZO1X, not LZO1Z, and will not work here regardless of
platform.

Platform support: **Windows is verified** (`liblzo2-2.dll`, confirmed loadable
and round-tripped through the real library in `cargo test`). Linux/macOS try
conventional `liblzo2` SONAMEs (`liblzo2.so.2`, `liblzo2.dylib`, etc. — see
`src/lzo.rs`) but this is unvalidated guesswork, not tested on those
platforms. If it fails to load there, check what your distro's package
actually installs and adjust the candidate list.

## Testing and validation performed

- **Unit test**: `lzo::tests::round_trips_through_the_real_dll` compresses and
  decompresses real data through the actual `liblzo2` on the build machine,
  empirically proving the FFI ABI assumptions (calling convention, `lzo_uint`
  width) rather than trusting documentation alone.
- **Cross-validation against the C++ reference binary**: 8 hand-built
  synthetic packets covering all 5 decodable message types, the
  `BookType != 1` skip path, multi-packet datagram framing, and malformed
  input — Rust output diffed byte-for-byte identical to the reference's own
  output on every case.
- **Not yet done**: validation against a real captured NSE packet. Everything
  above is synthetic, hand-constructed from the documented offsets — if those
  offsets are wrong in a way the synthetic packets don't expose, this crate
  and the C++ reference would agree and both be wrong. A real capture is the
  one input that would tighten the confidence levels below from UNKNOWN/
  UNVALIDATED to confirmed.

## Known limitations

- **17208** LTP/ATP/OHLC/TBQ/TSQ/total_traded_qty: real wire offsets unknown
  (every sample seen had these as all-zero bytes); always emitted as `0`
  rather than guessed.
- **17208** with `NoOfRecords > 1`: unvalidated record-to-record stride; only
  the first record is decoded, with a warning logged for the rest.
- **7305**: only 3 fields modeled; the real NSE message likely carries more
  trailing fields (tick size, freeze qty, board lot, ...).
- **7200**: not decoded at all — the real MBO record layout past `Token`
  isn't determinable from any usage seen so far.
- **LZO on Linux/macOS**: library name resolution is unvalidated (see above).
- The `BCSeqNo` field is read from header offset **+4**, not +8 as the
  reference's own `BCAST_HEADER` struct layout would imply — this is a
  deliberate, verified deviation (see `src/header.rs`), not an oversight.

## Design notes

Unlike the C++ reference (which casts raw pointers onto `#pragma pack(1)`
structs), this port reads every field explicitly via `byteorder` at named
offset constants. This avoids Rust's undefined-behavior-adjacent restrictions
on referencing misaligned fields of a packed struct, and makes every
hand-derived offset (several of which contradict what the "obvious" struct
layout would suggest) visible at the call site instead of hidden in a type
definition.

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or
  http://opensource.org/licenses/MIT)

at your option.
