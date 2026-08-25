//! LZO1Z decompression via runtime dynamic loading of `liblzo2` -- the same
//! library the reference C++ implementation links against on Windows
//! (`liblzo2-2.dll`).
//!
//! Most Rust LZO crates (`minilzo-rs`, `lzo1x`, ...) implement **LZO1X**, not
//! **LZO1Z** -- they will NOT correctly decompress this feed. No headers or
//! import library (`.lib`/`.dll.a`) for this LZO build are vendored anywhere,
//! so instead of a build.rs + bindgen setup, this binds directly against the
//! shared library's real exports via `libloading`. The exact export names
//! were confirmed with `objdump -p liblzo2-2.dll` against the copy shipped
//! alongside the MinGW toolchain used to build the C++ reference:
//! `lzo1z_decompress`, `lzo1z_decompress_safe`, `lzo1z_999_compress`, etc.
//!
//! ## Platform support: Windows verified, others best-effort/UNVALIDATED
//! `LZO_LIBRARY_CANDIDATES` below tries `liblzo2-2.dll` on Windows (the name
//! actually confirmed present and loadable on the machine this crate was
//! ported on -- see the round-trip test), and falls back through a list of
//! conventional `liblzo2` SONAMEs on Linux/macOS (`liblzo2.so.2`, etc.).
//! Those non-Windows names are standard *nix shared-library naming
//! conventions, NOT confirmed against a real liblzo2 install on those
//! platforms -- nobody has run this crate's test suite there. If loading
//! fails on Linux/macOS, check what your distro's `liblzo2`/`lzo` package
//! actually installs (e.g. `ldconfig -p | grep lzo2` on Linux) and add that
//! exact name to the candidate list for your platform.
//!
//! This uses `lzo1z_decompress_safe` (bounds-checked against the caller's
//! output capacity) rather than the reference's unchecked `lzo1z_decompress`:
//! the reference's fixed unchecked 2048-byte scratch buffer is a latent
//! overflow risk not worth porting as-is.
//!
//! ## The one real unknown: `lzo_uint` width
//! LZO's public headers typedef `lzo_uint` in spirit to match `size_t`, but on
//! Windows' LLP64 data model the fallback type it resolves to (`unsigned
//! long`) is only 32 bits even in a 64-bit build. This module binds
//! `lzo_uint` as `u32` accordingly. That assumption is checked empirically by
//! `round_trips_through_the_real_dll` below (`cargo test`), which compresses
//! and decompresses through this exact DLL and asserts byte-for-byte
//! equality -- if `lzo_uint` were actually 64-bit here, that call would
//! corrupt args/stack rather than round-tripping cleanly. If that test ever
//! fails on a different LZO build, widening `LzoUint` to `u64` is the first
//! thing to try.

use std::ffi::c_void;
use std::os::raw::c_int;
use std::sync::OnceLock;

use libloading::{Library, Symbol};
use thiserror::Error;

type LzoUint = u32;

/// Candidate library names to try, in order, for the current platform.
/// Windows' `liblzo2-2.dll` is the only entry actually verified to load (see
/// module docs above) -- the rest are unverified conventional guesses.
#[cfg(target_os = "windows")]
const LZO_LIBRARY_CANDIDATES: &[&str] = &["liblzo2-2.dll"];
#[cfg(target_os = "macos")]
const LZO_LIBRARY_CANDIDATES: &[&str] = &["liblzo2.2.dylib", "liblzo2.dylib"];
#[cfg(all(unix, not(target_os = "macos")))]
const LZO_LIBRARY_CANDIDATES: &[&str] = &["liblzo2.so.2", "liblzo2.so"];
#[cfg(not(any(target_os = "windows", unix)))]
const LZO_LIBRARY_CANDIDATES: &[&str] = &[];

const LZO_E_OK: c_int = 0;

#[derive(Debug, Error)]
pub enum LzoError {
    #[error("failed to load any of {candidates:?}: {last_error}")]
    LoadFailed {
        candidates: &'static [&'static str],
        last_error: String,
    },

    #[error("symbol {0} not found in {1}: {2}")]
    SymbolNotFound(&'static str, &'static str, String),

    #[error("length {0} does not fit the 32-bit lzo_uint ABI -- see src/lzo.rs")]
    LengthOverflow(usize),

    #[error("lzo1z_decompress_safe returned error code {0}")]
    DecompressFailed(c_int),
}

type DecompressSafeFn = unsafe extern "C" fn(
    src: *const u8,
    src_len: LzoUint,
    dst: *mut u8,
    dst_len: *mut LzoUint,
    wrkmem: *mut c_void,
) -> c_int;

/// The candidate name that actually loaded, alongside the open `Library` --
/// kept so error messages about missing symbols can name the real library.
struct LoadedLibrary {
    name: &'static str,
    lib: Library,
}

fn library() -> Result<&'static LoadedLibrary, LzoError> {
    static LIB: OnceLock<Result<LoadedLibrary, String>> = OnceLock::new();
    let cell = LIB.get_or_init(|| {
        let mut last_error = "no candidate library names for this platform".to_string();
        for &name in LZO_LIBRARY_CANDIDATES {
            match unsafe { Library::new(name) } {
                Ok(lib) => return Ok(LoadedLibrary { name, lib }),
                Err(e) => last_error = e.to_string(),
            }
        }
        Err(last_error)
    });
    cell.as_ref().map_err(|last_error| LzoError::LoadFailed {
        candidates: LZO_LIBRARY_CANDIDATES,
        last_error: last_error.clone(),
    })
}

fn decompress_safe_symbol(lib: &LoadedLibrary) -> Result<Symbol<'_, DecompressSafeFn>, LzoError> {
    unsafe { lib.lib.get(b"lzo1z_decompress_safe\0") }
        .map_err(|e| LzoError::SymbolNotFound("lzo1z_decompress_safe", lib.name, e.to_string()))
}

/// Decompresses `input` (LZO1Z-compressed bytes) into `output`, returning the
/// number of bytes written. Bounds-checked against `output.len()` by the
/// underlying `_safe` call -- never writes past it.
pub fn decompress_lzo1z(input: &[u8], output: &mut [u8]) -> Result<usize, LzoError> {
    let lib = library()?;
    let func = decompress_safe_symbol(lib)?;

    let src_len: LzoUint = input
        .len()
        .try_into()
        .map_err(|_| LzoError::LengthOverflow(input.len()))?;
    let mut dst_len: LzoUint = output
        .len()
        .try_into()
        .map_err(|_| LzoError::LengthOverflow(output.len()))?;

    let ret = unsafe {
        func(
            input.as_ptr(),
            src_len,
            output.as_mut_ptr(),
            &mut dst_len,
            std::ptr::null_mut(),
        )
    };

    if ret != LZO_E_OK {
        return Err(LzoError::DecompressFailed(ret));
    }

    Ok(dst_len as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    type CompressFn = unsafe extern "C" fn(
        src: *const u8,
        src_len: LzoUint,
        dst: *mut u8,
        dst_len: *mut LzoUint,
        wrkmem: *mut c_void,
    ) -> c_int;

    /// Empirically validates the `LzoUint = u32` ABI assumption and the
    /// calling convention by round-tripping real data through the actual
    /// `liblzo2-2.dll` on this machine -- compress with `lzo1z_999_compress`,
    /// decompress with our `decompress_lzo1z`, compare bytes. No captured NSE
    /// packet is needed for this: it only proves the FFI plumbing is sound,
    /// not that the framing offsets elsewhere in this crate are correct.
    #[test]
    fn round_trips_through_the_real_dll() {
        let lib = library().expect("liblzo2 must be loadable for this test to mean anything");
        let compress: Symbol<CompressFn> = unsafe { lib.lib.get(b"lzo1z_999_compress\0") }
            .expect("lzo1z_999_compress must be exported by liblzo2");

        let plain: Vec<u8> = b"NSE FO DECODER LZO1Z ROUNDTRIP SELF TEST. "
            .iter()
            .copied()
            .cycle()
            .take(4096)
            .collect();

        let mut compressed = vec![0u8; plain.len() + 4096];
        // LZO1Z_999_MEM_COMPRESS is documented around ~450KB; over-allocate
        // generously since this is a one-off test call, not production code.
        let mut wrkmem = vec![0u8; 4 * 1024 * 1024];
        let mut compressed_len: LzoUint = compressed.len() as LzoUint;

        let ret = unsafe {
            compress(
                plain.as_ptr(),
                plain.len() as LzoUint,
                compressed.as_mut_ptr(),
                &mut compressed_len,
                wrkmem.as_mut_ptr() as *mut c_void,
            )
        };
        assert_eq!(ret, LZO_E_OK, "lzo1z_999_compress failed with code {ret}");
        compressed.truncate(compressed_len as usize);
        assert!(
            compressed.len() < plain.len(),
            "expected the highly-repetitive test payload to actually compress"
        );

        let mut decompressed = vec![0u8; plain.len() + 64];
        let n = decompress_lzo1z(&compressed, &mut decompressed)
            .expect("decompress_lzo1z should succeed on our own freshly-compressed output");
        decompressed.truncate(n);

        assert_eq!(
            decompressed, plain,
            "round-trip through the real liblzo2-2.dll produced different bytes -- \
             check the lzo_uint width assumption (LzoUint) in src/lzo.rs"
        );
    }
}
