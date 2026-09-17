# How `nse_fo_decoder` works (plain-language guide)

This is the simple-words companion to [README.md](README.md), which has the
full technical detail. Read this first if you just want to understand what
the thing does and what you need to run it.

## What is this, in one sentence?

NSE (the stock exchange) constantly broadcasts small chunks of data over the
network — prices, order book updates, open interest — and this library turns
those raw bytes into readable data (like "NIFTY 25000 CE: best buy price is
149.75").

## Why is this needed at all?

The exchange doesn't send you a neat JSON file. It sends raw binary bytes
over UDP (a type of network packet), packed as tightly as possible for
speed. Nobody outside NSE has an official manual for this format — it had to
be figured out by studying an existing C++ program that already decodes it,
and copying its exact behavior. That's what "reverse-engineered" means here:
we know it works because it matches the known-good C++ program, not because
we have NSE's own spec sheet.

## How it works, step by step

Think of one incoming packet like a sealed envelope that can contain several
smaller envelopes inside it, and each small envelope can be scrambled
(compressed) to save space.

1. **A packet arrives.** It's just a chunk of bytes — no meaning yet.
2. **Read "how many messages are inside."** The first few bytes say how many
   smaller messages are bundled into this one packet.
3. **For each smaller message, check if it's compressed.** Some messages are
   squeezed smaller using a compression method called LZO1Z. If so, the code
   un-squeezes it first, using a small helper library (`liblzo2`) that's
   loaded from the machine at runtime — it isn't compiled directly into this
   crate.
4. **Read the header.** Every message starts with a short header that says
   two important things: *what kind* of message this is (a number called
   the "transaction code") and *how long* the message is.
5. **Hand it to the right parser based on that code.** For example, code
   `7208` means "market price update" and gets parsed by one specific
   function that knows exactly which bytes mean price, quantity, etc. Each
   message kind has its own parser because each one lays out its fields
   differently.
6. **Produce a clean Rust value.** The output isn't raw bytes anymore — it's
   a proper structured value (and can be turned into JSON) with named fields
   like `token`, `ltp` (last traded price), `entries` (the 5 buy/5 sell
   price levels), and so on.
7. **Repeat for every bundled message**, then return the full list.

That's the entire job: bytes in, one entry point (`decode_packet`), a list
of readable messages out.

## What kinds of messages does it understand?

| Code | What it means | How confident are we it's fully correct? |
|---|---|---|
| 7208 | Live price + order book (5 buy/5 sell levels) | High |
| 17208 | Same, "enhanced" version | Partly high, partly unknown |
| 7202 | Ticker / open interest update | Medium |
| 7305 | Price band update | Medium |
| 7220 | Trade execution band | Medium |
| 7200 | Old-style combined message | Not decoded at all (skipped on purpose) |

"Medium"/"unknown" isn't a bug warning — it means: we've only ever seen this
in a handful of sample packets, so trust it but don't assume every field is
100% correct without double-checking against a real capture.

## What do you need on your machine to build and run it?

- **Rust toolchain** (the normal `cargo`/`rustc` install) — nothing exotic,
  standard edition-2021 Rust.
- **No internet-facing setup needed to build it** — its dependencies
  (`byteorder`, `serde`, `serde_json`, `thiserror`, `libloading`) all come
  from crates.io automatically when you run `cargo build`.
- **`liblzo2` on your system, only if you want to decode compressed
  messages** (or run the self-test that proves the compression code works).
  On Windows this is `liblzo2-2.dll` somewhere Windows can find it (same
  folder as the program, or on `PATH`). Without it, everything still
  compiles and runs — you'd just get an error the moment a compressed
  message actually shows up. This has only been verified on **Windows**;
  Linux/macOS support is untested guesswork.
- **To test against real exchange data**, you either need:
  - a network connection that can actually reach NSE's multicast broadcast
    feed (this only works from specific, whitelisted network setups — most
    ordinary machines cannot reach it), or
  - a saved fixture file of previously captured packets, which is how it's
    normally tested day-to-day without needing the live feed at all.

## How do you actually use it?

As a library, from your own Rust code:

```toml
[dependencies]
nse_fo_decoder = { path = "path/to/nse_fo_decoder" }
```

```rust
let raw_datagram: &[u8] = /* one UDP packet's bytes */;
let messages = nse_fo_decoder::decode_packet(raw_datagram)?;
for msg in &messages {
    println!("{}", serde_json::to_string(msg)?);
}
```

There's also a ready-made command-line tool bundled in: feed it one
hex-encoded packet per line, get back one line of JSON per line of input.

## How do we know it's actually correct?

- **A self-test** round-trips real data through the actual compression
  library on the machine, proving the compression-handling code really
  works (not just "looks right on paper").
- **Cross-checked against the original C++ program**: the same input packets
  were run through both, byte for byte, and the outputs matched exactly.
- **Not yet done**: testing against a genuine, freshly captured NSE packet.
  Everything validated so far is hand-built test data — if the byte layout
  assumptions are wrong in some way that hand-built data doesn't expose,
  this library and the C++ program would both be wrong in the same way.

## How this project (`box_scanner_rust`) uses it for testing

This library was copied into `crates/nse_fo_decoder` inside this workspace
purely as a test bed, separate from wherever the library eventually lives on
its own. The app `apps/nse_fo_verify` was extended with two extra modes to
exercise it:

- `cargo run -p nse_fo_verify -- replay-rust` — decodes the saved sample
  packets through this library and prints the results, so you can see it
  working without needing the live feed.
- `cargo run -p nse_fo_verify -- live-rust` — same thing, but against the
  real multicast feed (only works if this machine can actually reach it).

Running `replay` (no `-rust`) does the same thing but through the original
C++ program instead, so the two can be compared side by side.
