// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! Logical operations: AND, ORA, EOR, BIT.
//!
//! AND, ORA and EOR combine the operand into A bit by bit and set Z and N
//! from the result; they are the same handler with a different operator,
//! which is what the book means by a twin. BIT is the odd one: it tests
//! `A & M` for Z without keeping the result, and copies bits 7 and 6 of the
//! memory byte into N and V, which is why hardware puts its status bits
//! there.
//!
//! Introduced in Build 7 — Interlude: A Filing System for the Match;
//! Build 9's cheatsheet sets the three operators side by side.

use crate::addressing::AddressingMode;
use crate::cpu::Chip6502;
use crate::{Bus, StatusFlag};

impl Chip6502 {
    /// AND: A ← A & M; sets Z, N.
    pub(crate) fn and<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        self.a &= bus.memory_read(addr);
        self.set_zero_and_negative_flags(self.a);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// ORA: A ← A | M; sets Z, N.
    pub(crate) fn ora<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        self.a |= bus.memory_read(addr);
        self.set_zero_and_negative_flags(self.a);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// EOR: A ← A ^ M; sets Z, N.
    pub(crate) fn eor<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        self.a ^= bus.memory_read(addr);
        self.set_zero_and_negative_flags(self.a);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// BIT: Z ← (A & M) == 0, N ← M bit 7, V ← M bit 6; A unchanged.
    pub(crate) fn bit<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        let operand = bus.memory_read(addr);
        self.bit_test(operand);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// The shared BIT logic: three flags from three separate conditions.
    #[rustfmt::skip]
    fn bit_test(&mut self, operand: u8) {
        let and_result = self.a & operand;
        self.set_flag(StatusFlag::Zero,     and_result == 0);
        self.set_flag(StatusFlag::Negative, operand & 0x80 != 0);   // from memory, not from A
        self.set_flag(StatusFlag::Overflow, operand & 0x40 != 0);   // from memory, not from A
        // A is NOT modified
    }
}
