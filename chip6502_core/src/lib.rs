// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! An NMOS 6502 processor core, built one pattern at a time.
//!
//! This crate is the reference implementation for *6502 Construction Set*:
//! all 151 documented opcodes, decimal mode with the NMOS flag behavior,
//! IRQ, NMI and `BRK`, and per-instruction cycle counts, passing Klaus
//! Dormann's functional, decimal and interrupt suites. The processor,
//! [`Chip6502`], owns only its registers; every memory access goes through
//! the [`Bus`] trait, which it borrows for one instruction at a time, so the
//! same core runs against plain RAM ([`FlatBus`]), a console with two ports
//! ([`ConsoleBus`]), or a whole machine. The book builds it in thirty steps,
//! each driven by a 6502 program; this crate is where they end up.
//!
//! See <https://leanpub.com/6502constructionset> for the ebook, or <https://www.silicon-arcade.com> for the paperback.
//!
//! # Example
//!
//! Load three instructions that print `H` and stop, and run them:
//!
//! ```
//! use chip6502_core::{Chip6502, ConsoleBus};
//!
//! let mut bus = ConsoleBus::new();
//! let mut cpu = Chip6502::new();
//! let program = [
//!     0xA9, 0x48,       // LDA #'H'
//!     0x8D, 0x01, 0xF0, // STA $F001  (the console output port)
//!     0x4C, 0x05, 0x02, // JMP $0205  (jump to self: halt)
//! ];
//! bus.ram.memory[0x0200..0x0200 + program.len()].copy_from_slice(&program);
//! cpu.pc = 0x0200;
//!
//! loop {
//!     let pc_before = cpu.pc;
//!     cpu.step(&mut bus); // returns the cycles the instruction took
//!     if cpu.pc == pc_before {
//!         break; // the program jumped to itself
//!     }
//! }
//! assert_eq!(bus.output_log, b"H");
//! assert_eq!(cpu.cycles, 2 + 4 + 3);
//! ```
//!
//! # Modules and where the book builds them
//!
//! The first reference to each chapter below uses the book's full form, as
//! in *6502 Construction Set*, Build 11 — Jump-table dispatch; the table
//! uses the short form.
//!
//! | Module | What it holds | Introduced in |
//! |---|---|---|
//! | `bus` | the [`Bus`] trait, the seam to memory | Chapter 4 |
//! | `bus_flat` | [`FlatBus`], 64 KB of plain RAM | Chapter 4 |
//! | `bus_console` | [`ConsoleBus`], RAM plus `$F001` out and `$F000` in | Chapter 4; input in Build 21 |
//! | `cpu` | [`Chip6502`]: registers, `reset`, `step`, interrupt lines | Chapter 4 |
//! | `flags` | [`StatusFlag`], the eight status bits | Chapter 5 |
//! | `cpu_helpers` | the stack helpers and the address resolvers | Build 2; resolvers Build 22 |
//! | `addressing` | the thirteen addressing modes | Build 3 |
//! | `disassemble` | [`disassemble_at`], [`disassemble_range`] | Build 3 |
//! | `cpu_ops_*` (11 files) | the instruction handlers, one family per file | Build 7 |
//! | `opcodes` | the `OPCODES` table, `dispatch`, [`instruction_len_at`] | Build 22 |
//!
//! The tests live in three more modules: `tests_unit` (one instruction at a
//! time, from Chapter 5), `tests_functional` (each build's program, from
//! Build 1) and `tests_kd` (the conformance suites, Build 26), plus the
//! integration test `tests/unimplemented_opcode.rs` (Build 28).
//!
//! # Running the tests
//!
//! `cargo test` runs the unit, functional and integration tests. The three
//! conformance suites are GPL-3.0, are not part of this repository, and are
//! `#[ignore]`d; with the binaries in a directory of your own:
//!
//! ```text
//! CHIP6502_TEST_ROMS=/path/to/roms cargo test --release -p chip6502_core klaus -- --ignored
//! ```
//!
//! # Failure model
//!
//! This is the core's error-handling policy, from Build 28.
//!
//! - **`panic!` is for contract violations only**: invariants internal to
//!   this crate, unreachable by any sequence of public API calls with any
//!   data. The message names the broken invariant and carries the opcode and
//!   PC that locate it. There are two such sites: `resolve_addr` meeting a
//!   mode its table row should never have given it, and `not_implemented`
//!   in this crate's own test builds.
//! - **Nothing an emulated program does may panic in a normal build.** An
//!   undocumented opcode, a wild jump, self-modifying code or a stack that
//!   wraps is input, not a bug: the silicon has a behavior for all of them.
//!   An opcode the core does not model is recorded for
//!   [`Chip6502::last_unimplemented_opcode`] and PC stops advancing.
//! - **No `Result` on the hot path** for errors that cannot occur. Every
//!   input is a typed value that is valid by construction, so `step` returns
//!   plain values.
//! - **Checks the type system already makes are deleted**, not kept "for
//!   safety": a `u16` address cannot overrun a `[u8; 65536]`.
//! - **`unwrap` and `expect` appear only in tests and `main`.**
//!
//! The core owns no logger and prints nothing on any path a program can
//! reach; state is inspectable instead (public registers,
//! [`Chip6502::cycles`], `last_unimplemented_opcode`). The one deliberate
//! exception is [`disassemble_range`], a debugging aid that prints a
//! listing because that is its job.

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![warn(missing_docs)]

mod bus;
pub use bus::Bus;

mod bus_flat;
pub use bus_flat::FlatBus;

mod cpu;
pub use cpu::Chip6502;

mod bus_console;
pub use bus_console::{ConsoleBus, INPUT_PORT, OUTPUT_PORT};

mod flags;
pub use flags::StatusFlag;

mod cpu_helpers;

mod cpu_ops_arithmetic;
mod cpu_ops_branches;
mod cpu_ops_flags;
mod cpu_ops_incdec;
mod cpu_ops_jumps;
mod cpu_ops_loadstore;
mod cpu_ops_logical;
mod cpu_ops_regtransfer;
mod cpu_ops_shifts;
mod cpu_ops_stack;
mod cpu_ops_system;

mod addressing;
mod disassemble;
pub use disassemble::{disassemble_at, disassemble_range};

mod opcodes;
pub use opcodes::instruction_len_at;

#[cfg(test)]
mod tests_unit;

#[cfg(test)]
mod tests_functional;

#[cfg(test)]
mod tests_kd;
