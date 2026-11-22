// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! `Chip6502`: the registers, reset, and the fetch–decode–execute step.
//!
//! The processor is six public register fields and a `step` method that
//! borrows a [`Bus`] for one instruction execution cycle; it never owns its memory,
//! which is what lets the same CPU run against plain RAM in a test, a simple MMIO backed bus for
//! a development harness, and a whole machine later. PC stays parked on the opcode
//! byte for the whole of an instruction and each handler advances it once, explicitly, at the end,
//! so a disassembly listing and the CPU agree about where an instruction
//! starts. `step` checks the interrupt lines, fetches, has
//! `decode_and_execute` make the single call into `dispatch`, and returns the
//! instruction's cycle count.
//!
//! Introduced in Chapter 4 — Rust and System Programming; decode moves to the table
//! in Build 22, interrupts arrive in Build 23, cycle counting in Build 27 and the failure record in Build 28.

use crate::opcodes::OPCODES;
use crate::{Bus, StatusFlag};

const DEBUG: bool = false; // flip to true for an instruction trace

/// An NMOS 6502 processor: its registers, and nothing else.
pub struct Chip6502 {
    /// Accumulator - main register for arithmetic and logic.
    pub a: u8,
    /// X register - indexing and loop counters.
    pub x: u8,
    /// Y register - indexing and loop counters.
    pub y: u8,
    /// Stack pointer - next free byte in the stack (0x0100 - 0x01FF).
    pub sp: u8,
    /// Program counter - address of the next instruction to execute.
    pub pc: u16,
    /// Processor status (flags) - N, V, B, D, I, Z, C.
    pub p: u8,
    /// Total clock cycles executed since the last reset.
    pub cycles: u64,
    /// Penalty cycles the current instruction has earned (branches write
    /// here); cleared before each dispatch.
    pub(crate) last_step_cycles: u8,
    /// Set by the indexed resolvers when base + index changed page; charged
    /// by `step` only when the opcode's row is page-sensitive. Cleared
    /// before each dispatch.
    pub(crate) page_crossed: bool,
    /// The IRQ line is asserted. Level-triggered: it stays set until the
    /// host calls [`clear_irq`](Self::clear_irq).
    pub(crate) irq_pending: bool,
    /// An NMI edge has arrived and not yet been taken. Edge-triggered: the
    /// CPU clears it when it takes the NMI.
    pub(crate) nmi_pending: bool,
    /// The last opcode the core met but does not model, and its address,
    /// recorded by `not_implemented` in a normal build (Build 28).
    pub(crate) last_unimplemented_opcode: Option<(u8, u16)>,
}

impl Chip6502 {
    /// A fresh CPU in a deterministic zeroed state. Call `reset`
    /// afterwards to apply the NMOS startup defaults.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "kept as the plain `fn` the book prints in Chapter 4"
    )]
    #[expect(
        clippy::use_self,
        reason = "the printed listing names the type in its constructor"
    )]
    #[rustfmt::skip]
    pub fn new() -> Self {
        Chip6502 {
            a: 0, x: 0, y: 0, sp: 0, pc: 0, p: 0,
            cycles: 0,
            last_step_cycles: 0,
            page_crossed: false,
            irq_pending: false,
            nmi_pending: false,
            last_unimplemented_opcode: None,
        }
    }

    /// Hardware reset: load PC from the vector at `$FFFC/$FFFD` (low
    /// byte first), set I and the always-on bit 5, and consume the
    /// three bytes of stack the silicon's dummy pushes would. D is
    /// undefined after a real NMOS reset; this core clears it so every
    /// run starts the same.
    #[rustfmt::skip]
    pub fn reset<B: Bus>(&mut self, bus: &mut B) {
        let low  = u16::from(bus.memory_read(0xFFFC));
        let high = u16::from(bus.memory_read(0xFFFD));
        self.pc = (high << 8) | low;
        self.sp = self.sp.wrapping_sub(3);
        self.p = 0b0010_0100;   // I = 1, bit 5 = 1, the rest clear: $24
        self.cycles = 0;        // the count is "since reset" (Build 27)
        self.last_unimplemented_opcode = None;   // no stale record survives a reset
    }

    /// The last opcode an emulated program ran that this core does not
    /// model, and the address it was at; `None` since `new` or `reset`.
    /// When PC stops moving, this says whether the program halted itself or
    /// met such an opcode.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "kept as the plain `fn` the book prints in Build 28"
    )]
    pub fn last_unimplemented_opcode(&self) -> Option<(u8, u16)> {
        self.last_unimplemented_opcode
    }

    /// Assert the IRQ line. The CPU takes the interrupt at the next
    /// instruction boundary at which I is clear, and keeps taking it until
    /// the line is released with [`clear_irq`](Self::clear_irq).
    pub const fn request_irq(&mut self) {
        self.irq_pending = true;
    }

    /// Release the IRQ line, as a device does once its interrupt has been
    /// acknowledged.
    pub const fn clear_irq(&mut self) {
        self.irq_pending = false;
    }

    /// Signal an NMI edge. The CPU takes it at the next instruction boundary
    /// whatever I says, and clears it as it does.
    pub const fn trigger_nmi(&mut self) {
        self.nmi_pending = true;
    }

    /// Execute one instruction, or take one pending interrupt, and return
    /// the clock cycles it took on an NMOS 6502.
    ///
    /// Interrupts are recognized only here, between instructions: NMI first,
    /// then IRQ if I is clear; either entry costs 7 cycles. An instruction
    /// costs its `OPCODES` base, plus 1 for a page-sensitive read whose index
    /// crossed a page, plus a taken branch's 1 or 2. The total is also added
    /// to [`cycles`](Self::cycles).
    ///
    /// # Panics
    ///
    /// Only on a broken invariant inside this crate (an `OPCODES` row whose
    /// mode its handler cannot resolve), never on anything a program does:
    /// an opcode the core does not model is recorded for
    /// [`last_unimplemented_opcode`](Self::last_unimplemented_opcode) and PC
    /// stays where it is. (This crate's own tests panic there instead.)
    pub fn step<B: Bus>(&mut self, bus: &mut B) -> u8 {
        // Interrupt entries bypass the opcode table, so they charge the
        // accumulator themselves.
        if self.nmi_pending {
            self.take_nmi(bus);
            self.cycles += 7;
            return 7;
        }
        if self.irq_pending && !self.flag(StatusFlag::InterruptDisable) {
            self.take_irq(bus);
            self.cycles += 7;
            return 7;
        }
        if DEBUG {
            println!("{}", crate::disassemble::disassemble_at(self.pc, bus));
        }
        let opcode = bus.memory_read(self.pc);
        let info = &OPCODES[usize::from(opcode)];

        self.last_step_cycles = 0; // reset BEFORE dispatch —
        self.page_crossed = false; // stale values from the last instruction must not leak
        self.decode_and_execute(opcode, bus);
        let mut total = info.cycles + self.last_step_cycles; // branch penalties
        if info.page_sensitive && self.page_crossed {
            total += 1; // indexed-read page-cross
        }
        self.cycles += u64::from(total);
        total
    }

    /// Execute the instruction whose opcode byte, already fetched from PC,
    /// is `opcode`: look up its addressing mode in `OPCODES` and dispatch.
    ///
    /// # Panics
    ///
    /// Only on a broken invariant inside this crate; see [`step`](Self::step).
    pub(crate) fn decode_and_execute<B: Bus>(&mut self, opcode: u8, bus: &mut B) {
        let info = &OPCODES[usize::from(opcode)];
        crate::opcodes::dispatch(opcode, self, info.addressing_mode, bus);
    }

    /// Set Z if `result` is zero and N from its bit 7; clear them otherwise.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "kept as the plain `fn` the book prints in Chapter 5"
    )]
    pub(crate) fn set_zero_and_negative_flags(&mut self, result: u8) {
        if result == 0 {
            self.p |= StatusFlag::Zero as u8;
        } else {
            self.p &= !(StatusFlag::Zero as u8);
        }
        if result & 0b1000_0000 != 0 {
            self.p |= StatusFlag::Negative as u8;
        } else {
            self.p &= !(StatusFlag::Negative as u8);
        }
    }

    /// Set or clear one status flag from a `bool`.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "kept as the plain `fn` the book prints in Build 15"
    )]
    pub(crate) fn set_flag(&mut self, f: StatusFlag, on: bool) {
        if on {
            self.p |= f as u8;
        } else {
            self.p &= !(f as u8);
        }
    }

    /// True if flag `f` is set in the live status register.
    pub(crate) const fn flag(&self, f: StatusFlag) -> bool {
        self.p & f as u8 != 0
    }
}

impl Default for Chip6502 {
    fn default() -> Self {
        Self::new()
    }
}
