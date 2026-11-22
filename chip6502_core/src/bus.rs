// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! The seam between the processor and everything it is wired to.
//!
//! Every byte the CPU fetches, loads or stores passes through these two
//! methods, so a type that implements them is a machine the CPU can run on:
//! a bare 64 KB array in the tests, a console with two ports, or later a
//! whole BBC Micro. The key decision is that the CPU borrows its bus for one
//! instruction rather than owning it, which keeps the processor free of any
//! machine knowledge and lets one test hand it plain RAM. `memory_read` takes
//! `&mut self` so that a read can have side effects, as hardware registers
//! do.
//!
//! Introduced in Chapter 4 — Rust and System Programming; the reasons
//! it is a parameter rather than a field close that chapter.

/// A 6502 address bus: one byte read or written at one 16-bit address.
pub trait Bus {
    /// Read the byte at `addr`. Takes `&mut self` because a device behind
    /// the address may change state when it is read.
    fn memory_read(&mut self, addr: u16) -> u8;
    /// Write `value` to `addr`.
    fn memory_write(&mut self, addr: u16, value: u8);
}
