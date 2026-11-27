//! Steps over two things the title does that Windows would end the process
//! on but the console's kernel tolerates:
//!
//! 1. **Writes to the null page** (address 0). Grand Theft Auto VI 1.0.2.1
//!    stores a magic there (`mov dword ptr [0], 0x4d8bd421`), which faults on
//!    Windows (0xc0000005, "write 0x0") because address 0 is unmapped (the
//!    low 64 KiB is reserved and cannot be mapped). The store is discarded.
//! 2. **The title's debug breakpoint** (`mov dword ptr [0], 0; int3`): with
//!    no debugger the console just continues past the `int3`, but Windows
//!    raises a fatal `EXCEPTION_BREAKPOINT` (0x80000003). The `int3` is one
//!    byte and is stepped over.
//!
//! A vectored handler sees the fault before any other and advances the
//! instruction pointer, the same way the console-instruction handler steps
//! over `extrq`/`insertq` ([`crate::console_insn`]). Only the title's own
//! images are touched; an `int3` in our code stays fatal.

use std::sync::atomic::{AtomicU64, Ordering};

use windows_sys::Win32::Foundation::{
    EXCEPTION_ACCESS_VIOLATION, EXCEPTION_BREAKPOINT, HMODULE, NTSTATUS,
};
use windows_sys::Win32::System::Diagnostics::Debug::{
    AddVectoredExceptionHandler, EXCEPTION_CONTINUE_EXECUTION, EXCEPTION_CONTINUE_SEARCH,
    EXCEPTION_POINTERS,
};
use windows_sys::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
    GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
};

/// Null-page stores stepped over since the start.
static SKIPPED: AtomicU64 = AtomicU64::new(0);
/// Title breakpoints (`int3`) stepped over since the start.
static BROKEN: AtomicU64 = AtomicU64::new(0);

/// Registers the handler. `VICTORY_NULL_PAGE=0` leaves the title to fault.
pub fn install() {
    if std::env::var_os("VICTORY_NULL_PAGE").is_some_and(|v| v == "0") {
        return;
    }
    // SAFETY: the handler only steps over a null-page store or a title `int3`.
    unsafe { AddVectoredExceptionHandler(1, Some(on_fault)) };
}

unsafe extern "system" fn on_fault(info: *mut EXCEPTION_POINTERS) -> i32 {
    // SAFETY: the system passes the exception being dispatched.
    let record = unsafe { &*(*info).ExceptionRecord };
    let ip = record.ExceptionAddress as usize;

    // The title's debug breakpoint: `int3` in one of its images, stepped over
    // so the title continues as it does on the console with no debugger.
    if record.ExceptionCode == EXCEPTION_BREAKPOINT as NTSTATUS && in_title_image(ip) {
        // SAFETY: the context the system passes. `int3` is one byte; the saved
        // instruction pointer is already past it on this path, so only step
        // when it is still on it.
        let context = unsafe { (*info).ContextRecord };
        let rip = unsafe { (*context).Rip } as usize;
        let past = if rip > 0 && unsafe { *((rip - 1) as *const u8) } == 0xCC {
            rip
        } else if unsafe { *(rip as *const u8) } == 0xCC {
            rip + 1
        } else {
            return EXCEPTION_CONTINUE_SEARCH;
        };
        unsafe { (*context).Rip = past as u64 };
        let n = BROKEN.fetch_add(1, Ordering::Relaxed) + 1;
        if n == 1 || n.is_multiple_of(1000) {
            tracing::info!(ip = format_args!("{ip:#x}"), "stepped over a title breakpoint (int3; the {n}-th)");
        }
        return EXCEPTION_CONTINUE_EXECUTION;
    }

    // A write to the null page.
    const WRITE: usize = 1;
    if record.ExceptionCode != EXCEPTION_ACCESS_VIOLATION as NTSTATUS
        || record.NumberParameters < 2
        || record.ExceptionInformation[0] != WRITE
    {
        return EXCEPTION_CONTINUE_SEARCH;
    }
    // The null page only; anything above it is some other fault.
    let address = record.ExceptionInformation[1];
    if address >= 0x1000 {
        return EXCEPTION_CONTINUE_SEARCH;
    }
    let Some(len) = instruction_len(ip) else {
        return EXCEPTION_CONTINUE_SEARCH;
    };
    // SAFETY: the context the system passes, to go on from the next
    // instruction. The discarded store leaves the page unwritten, as the
    // title never reads it back.
    unsafe { (*(*info).ContextRecord).Rip = (ip + len) as u64 };
    let n = SKIPPED.fetch_add(1, Ordering::Relaxed) + 1;
    if n == 1 || n.is_multiple_of(1000) {
        tracing::info!(ip = format_args!("{ip:#x}"), "stepped over a write to the null page (the {n}-th)");
    }
    EXCEPTION_CONTINUE_EXECUTION
}

/// Whether `address` lies in a title image (`victory-image-*.dll`), as
/// opposed to our own code, whose breakpoints must not be stepped over.
fn in_title_image(address: usize) -> bool {
    let mut module: HMODULE = std::ptr::null_mut();
    // SAFETY: looks up the module containing `address`; no reference taken.
    let found = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            address as *const u16,
            &mut module,
        )
    };
    if found == 0 {
        return false;
    }
    let mut path = [0u16; 260];
    // SAFETY: `module` is loaded; the buffer length is passed.
    let len = unsafe { GetModuleFileNameW(module, path.as_mut_ptr(), path.len() as u32) };
    let name = String::from_utf16_lossy(&path[..len as usize]);
    name.rsplit('\\').next().is_some_and(|n| n.starts_with("victory-image"))
}

/// The length in bytes of the x86-64 instruction at `ip`, decoded just far
/// enough to step over a store to an absolute low address; `None` when the
/// bytes are not a form the handler steps over (the fault is then left to
/// the usual crash path).
fn instruction_len(ip: usize) -> Option<usize> {
    // SAFETY: `ip` is the faulting instruction pointer, which points at code.
    let code = unsafe { std::slice::from_raw_parts(ip as *const u8, 15) };
    let mut i = 0;
    // Legacy prefixes and the REX byte.
    while let Some(&b) = code.get(i) {
        match b {
            0x66 | 0x67 | 0xF2 | 0xF3 | 0xF0 | 0x2E | 0x36 | 0x3E | 0x26 | 0x64 | 0x65 | 0x40..=0x4F => {
                i += 1;
            }
            _ => break,
        }
    }
    // The one-byte opcodes that store an immediate or a register to a
    // ModRM memory operand; the two-byte maps (0F ...) are not stepped here.
    let op = code.get(i).copied()?;
    i += 1;
    let immediate = match op {
        0x88 | 0x89 => 0,               // mov r/m, reg
        0xC6 => 1,                      // mov r/m8, imm8
        0xC7 => 4,                      // mov r/m, imm32 (imm16 with 0x66)
        0x80 | 0x83 => 1,               // group 1 r/m, imm8
        0x81 => 4,                      // group 1 r/m, imm32
        _ => return None,
    };
    let modrm = code.get(i).copied()?;
    i += 1;
    let (mode, rm) = (modrm >> 6, modrm & 7);
    if mode != 3 {
        let mut base = rm;
        if rm == 4 {
            base = code.get(i).copied()? & 7;
            i += 1;
        }
        i += match mode {
            0 if base == 5 => 4, // disp32 (SIB with no base, or [rip+disp32])
            0 => 0,
            1 => 1,
            _ => 4,
        };
    }
    Some(i + immediate)
}
