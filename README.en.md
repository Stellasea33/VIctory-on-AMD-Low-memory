# VIctory-on-AMD-Low-memory

A compatibility layer (VIctory) that runs the Xbox Series build of GTA 6 natively on Windows, with some optimizations for AMD GPUs and low-RAM (16 GB) machines.

> Based on [Belobug/VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD) (the AMD RX 7900 branch).

---

## Hardware Requirements

| Item | Minimum | Recommended |
|---|---|---|
| CPU | AVX2 (Intel 12th Gen+ / AMD Ryzen 5000+) | — |
| GPU | AMD RDNA3 (RX 7000 series, 20 GB VRAM) | RX 7900 XT/XTX |
| RAM | 16 GB | **32 GB+** |
| OS | Windows 10/11 | — |
| **Resizable BAR** | Recommended (in BIOS) | — |

> ⚠️ **16 GB RAM is the hard bottleneck**: with ReBAR off you hit OOM (~3 min), with ReBAR on the game borrows VRAM and runs stably for 20+ minutes. For smooth play, 32 GB+ is still recommended.

---

## Prerequisites

### 1. PC FSR DLL (only needed for 2K/4K output)
2K/4K output uses the host's PC FSR upscaler (`amd_fidelityfx_upscaler_dx12.dll`).

**This package already ships the DLL**: to stay under GitHub's single-file upload limit, the upscaler is stored compressed (`fidelityfx/amd_fidelityfx_upscaler_dx12.dll.z`) and `cargo build` decompresses it to `amd_fidelityfx_upscaler_dx12.dll` automatically (see `apps/victory/build.rs`). `victory.toml` is pre-configured with `fidelityfx_dir = 'fidelityfx'`, so it works right after building — no extra download.

### 2. Shader Pre-warm (strongly recommended)
Pre-translate all shaders before the first run so the game does not compile them on the fly:

```powershell
.\target\diag\victory.exe shaders
```

Takes ~2.5 minutes; afterwards the shader cache hit rate is 100% (`misses=0` in the log).

---

## Build

Requires the Rust toolchain plus MSVC build tools (Visual Studio Build Tools):

```powershell
cargo build --profile diag -p victory --features diagnostics
```

Output: `target\diag\victory.exe`

---

## Configuration (victory.toml)

```toml
[paths]
game_dir = 'D:\GTA6'                                        # your game folder
fidelityfx_dir = 'fidelityfx'                               # PC FSR DLL folder (already shipped)
```

---

## Launch Modes

All modes run in PowerShell and automatically enable the memory optimizations (residency admission control + ReBAR VRAM offload + OOM retry + null-page/breakpoint fault tolerance).

### Mode 1: 1K output (1920×1080) — most stable, recommended

```powershell
.\scripts\play-lowmem.ps1 -Exe "D:\GTA6\game_scarlett_master.exe" -ConsoleProfile series-s -Upscaler 6
```

| Item | Detail |
|---|---|
| Profile | Series S (title memory ~8 GB) |
| Upscaler | Title's own (`-Upscaler 6`, avoids FSR scan-line artifacts) |
| Frame rate | 19 ~ 29 fps (smoothest) |
| Memory pressure | Lowest |

### Mode 2: 2K output (2560×1440) — balance of quality and frame rate

```powershell
.\scripts\play-lowmem.ps1 -Exe "D:\GTA6\game_scarlett_master.exe" -ConsoleProfile series-x -More @{VICTORY_FSR_4K='quality'; VICTORY_OUTPUT='2560x1440'}
```

If you want to try 4K, change 2560x1440 to 3840x2160 instead.

## Code Changes

| File | Change |
|---|---|
| `crates/victory-loader/src/null_page.rs` (new) | VEH handler for null-page writes and int3 breakpoints; fixes the 0xc0000005 crash |
| `crates/victory-gpu/src/host.rs` | OOM retry for upload/shared buffers; shader inputs in VRAM (`local_gpu_inputs`, ported from DexNom) |
| `crates/victory-gpu/src/descriptor/host.rs` | Shader-input allocation sites routed to VRAM |
| `apps/victory/src/run.rs` | Install null-page handler; FSR 4K mode supports a custom output resolution |
| `crates/victory-loader/src/lib.rs` | Export the null-page handler |
| `scripts/play-lowmem.ps1` (new) | Low-memory launcher (residency + ReBAR offload + resolution/upscaler options) |

---

## Known Issues

- **Tree/vegetation flicker**: a rendering-correctness issue (culling/LOD shader translation precision), part of the upstream "fix rendering" frontier; not solvable by the memory optimizations in this package.
- **FSR upscaling shows scan-line artifacts**: inherent to FSR 3.1 temporal upscaling; avoid by using the title's own upscaler (Mode 1).

## Disclaimer

This package is for study and research only. Use a game copy you legally own.
