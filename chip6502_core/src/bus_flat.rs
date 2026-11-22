// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! `FlatBus`: 64 KB of plain RAM, where no address is special.
//!
//! This is the bus the unit tests and the conformance suites run on. The
//! name describes the design of the address map, one uniform stretch of RAM,
//! rather than who uses it, because later buses wrap it and the BBC Micro's
//! map is quite complex. Indexing the `[u8; 65536]` array with a
//! `u16` cannot go out of range, so the bounds are guaranteed by the types
//! rather than by a runtime check. Introduced in Chapter 4 — Rust and System
//! Programming; Build 26, compliance testing, relies on the bus having no ports.

use crate::Bus;

/// Sixty-four kilobytes of read/write memory with no memory-mapped devices.
pub struct FlatBus {
    /// The whole 6502 address space, `$0000` to `$FFFF`. Tests load
    /// programs and inspect results through this field directly.
    pub memory: [u8; 65536],
}

impl FlatBus {
    /// A bus whose every byte is zero.
    #[must_use]
    #[expect(
        clippy::large_stack_arrays,
        reason = "the printed listing builds the 64 KB array by value; moving it to the heap would change FlatBus's public field type"
    )]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "kept as the plain `fn` the book prints in Chapter 4"
    )]
    #[expect(
        clippy::use_self,
        reason = "the printed listing names the type in its constructor"
    )]
    pub fn new() -> Self {
        FlatBus { memory: [0; 65536] }
    }
}

impl Default for FlatBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Bus for FlatBus {
    fn memory_read(&mut self, addr: u16) -> u8 {
        self.memory[usize::from(addr)]
    }

    fn memory_write(&mut self, addr: u16, value: u8) {
        self.memory[usize::from(addr)] = value;
    }
}
