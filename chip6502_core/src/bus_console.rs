// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! `ConsoleBus`: RAM plus a memory-mapped console, one port each way.
//!
//! `ConsoleBus` simulates MMIO behaviors for input and output. A write to [`OUTPUT_PORT`] (`$F001`)
//! is captured in `output_log` (for printing) and a read of [`INPUT_PORT`] (`$F000`) pops the next byte
//! from `input_queue` (for keyboard entry), or returns 0 when nothing is queued; every other
//! address is the wrapped `FlatBus`. The ports live on a separate type rather
//! than inside `FlatBus` because deciding that an address is a device is a
//! machine wide decision, and the plain-RAM bus makes none; and the CPU is unaware
//! that the ports even exist. Output is captured rather than printed, so a test can
//! assert on it and an application can decide where it goes.
//!
//! Introduced in Chapter 4 — Rust and System Programming; the input port arrives in Build 21.

use std::collections::VecDeque;

use crate::bus_flat::FlatBus;
use crate::Bus;

/// MMIO address the emulated program writes to emit a byte of output.
pub const OUTPUT_PORT: u16 = 0xF001;

/// Address the emulated program reads to get a byte of input.
pub const INPUT_PORT: u16 = 0xF000;

/// A 64 KB RAM bus with a console output port at [`OUTPUT_PORT`] and an
/// input port at [`INPUT_PORT`].
pub struct ConsoleBus {
    /// The underlying memory. Load programs through `bus.ram.memory`.
    pub ram: FlatBus,
    /// Bytes waiting to be read from [`INPUT_PORT`], oldest first.
    pub input_queue: VecDeque<u8>,
    /// Every byte written to [`OUTPUT_PORT`], in order.
    pub output_log: Vec<u8>,
}

impl ConsoleBus {
    /// A console bus with zeroed RAM, no queued input and an empty output log.
    #[must_use]
    #[rustfmt::skip]
    #[expect(
        clippy::use_self,
        reason = "the printed listing names the type in its constructor"
    )]
    pub fn new() -> Self {
        ConsoleBus { ram: FlatBus::new(), input_queue: VecDeque::new(), output_log: Vec::new() }
    }

    /// Queue one byte for the emulated program to read from
    /// [`INPUT_PORT`] — one keystroke.
    pub fn queue_input_byte(&mut self, byte: u8) {
        self.input_queue.push_back(byte);
    }

    /// Queue a whole slice — the equivalent of typing a string.
    pub fn synthesize_queue_input(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.queue_input_byte(byte);
        }
    }
}

impl Default for ConsoleBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Bus for ConsoleBus {
    fn memory_read(&mut self, addr: u16) -> u8 {
        match addr {
            // An empty queue reads as 0 — what the monitor spins on.
            INPUT_PORT => self.input_queue.pop_front().unwrap_or(0),
            _ => self.ram.memory_read(addr),
        }
    }

    // Unchanged from Chapter 4 — output was already an MMIO intercept;
    // input is the same trick pointed the other way.
    fn memory_write(&mut self, addr: u16, value: u8) {
        match addr {
            OUTPUT_PORT => self.output_log.push(value),
            _ => self.ram.memory_write(addr, value),
        }
    }
}
