//! Decompress the FSR upscaler DLL at build time.
//!
//! `fidelityfx/amd_fidelityfx_upscaler_dx12.dll` is ~28 MB, over GitHub's
//! upload limit, so the repository ships the zlib-compressed
//! `amd_fidelityfx_upscaler_dx12.dll.z` instead. This script unpacks it
//! next to the compressed file (into the project-root `fidelityfx/` folder)
//! before `victory.exe` is built, so a fresh clone is runnable after
//! `cargo build` with no extra step.

use std::path::{Path, PathBuf};

fn main() {
    // CARGO_MANIFEST_DIR is apps/victory; the repository root is two up.
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let Some(root) = manifest.parent().and_then(Path::parent) else {
        return;
    };
    let packed = root.join("fidelityfx").join("amd_fidelityfx_upscaler_dx12.dll.z");
    let dll = root.join("fidelityfx").join("amd_fidelityfx_upscaler_dx12.dll");

    if !packed.exists() {
        return; // nothing shipped (already unpacked, or no fidelityfx)
    }
    if dll.exists() && fresh(&packed, &dll) {
        return; // up to date
    }
    if let Err(why) = unpack(&packed, &dll) {
        panic!("unpacking {} failed: {why}", packed.display());
    }
    println!("cargo:warning=unpacked {} -> {}", packed.display(), dll.display());
}

/// Whether `dll` is at least as new as `packed` (so nothing to do).
fn fresh(packed: &Path, dll: &Path) -> bool {
    let (Ok(p), Ok(d)) = (std::fs::metadata(packed), std::fs::metadata(dll)) else {
        return false;
    };
    let (Ok(pt), Ok(dt)) = (p.modified(), d.modified()) else {
        return false;
    };
    dt >= pt
}

/// Inflate the zlib stream in `packed` and write it to `dll`.
fn unpack(packed: &Path, dll: &Path) -> Result<(), String> {
    let input = std::fs::read(packed).map_err(|e| e.to_string())?;
    // zlib format (the `.z` file is Python `zlib.compress`, i.e. a zlib header
    // followed by a deflate stream).
    let mut state = zlib_rs::Inflate::new(true, 15);
    let mut out: Vec<u8> = Vec::new();
    let (mut in_pos, mut out_pos) = (0usize, 0usize);
    loop {
        if out_pos == out.len() {
            let next = if out.is_empty() {
                input.len().saturating_mul(4).max(1 << 20)
            } else {
                out.len().saturating_mul(2)
            };
            out.resize(next, 0);
        }
        let status = state
            .decompress(&input[in_pos..], &mut out[out_pos..], zlib_rs::InflateFlush::NoFlush)
            .map_err(|e| format!("{e:?}"))?;
        let next_in = state.total_in() as usize;
        let next_out = state.total_out() as usize;
        match status {
            zlib_rs::Status::StreamEnd => {
                out.truncate(next_out);
                break;
            }
            zlib_rs::Status::Ok if next_in != in_pos || next_out != out_pos => {
                in_pos = next_in;
                out_pos = next_out;
            }
            zlib_rs::Status::Ok | zlib_rs::Status::BufError => {
                return Err("stream stalled (corrupt input?)".into());
            }
        }
    }
    std::fs::write(dll, out).map_err(|e| e.to_string())
}
