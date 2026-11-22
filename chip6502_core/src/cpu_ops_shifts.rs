// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! Shifts and rotates: ASL, LSR, ROL, ROR.
//!
//! Each moves a byte one bit left or right, sends the bit that falls off
//! into C, and sets Z and N from the result. The rotates also bring the old
//! carry in at the other end, which is how a shift spreads across several
//! bytes; the one rule to get right is to sample the incoming carry before
//! the outgoing bit overwrites it. The accumulator forms have handlers of
//! their own, since they have no address to resolve; the memory forms are
//! generic read-modify-write handlers that write the result once (Build 6
//! records why).
//!
//! Introduced in Build 7 — Interlude: A Filing System for the Match; the generic memory forms arrive in Build 22.

use crate::addressing::AddressingMode;
use crate::cpu::Chip6502;
use crate::{Bus, StatusFlag};

impl Chip6502 {
    /// ASL A (`$0A`): A ← A << 1, bit 7 → C; sets Z, N, C.
    pub(crate) fn asl_accumulator<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.a = self.asl_value(self.a);
        self.pc = self.pc.wrapping_add(1);
    }

    /// ASL on memory: M ← M << 1, bit 7 → C; sets Z, N, C.
    pub(crate) fn asl<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        let result = self.asl_value(bus.memory_read(addr));
        bus.memory_write(addr, result);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// LSR A (`$4A`): A ← A >> 1, bit 0 → C, N cleared; sets Z, C.
    pub(crate) fn lsr_accumulator<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.a = self.lsr_value(self.a);
        self.pc = self.pc.wrapping_add(1);
    }

    /// LSR on memory: M ← M >> 1, bit 0 → C, N cleared; sets Z, C.
    pub(crate) fn lsr<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        let result = self.lsr_value(bus.memory_read(addr));
        bus.memory_write(addr, result);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// ROL A (`$2A`): A ← (A << 1) | C, old bit 7 → C; sets Z, N, C.
    pub(crate) fn rol_accumulator<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.a = self.rol_value(self.a);
        self.pc = self.pc.wrapping_add(1);
    }

    /// ROL on memory: M ← (M << 1) | C, old bit 7 → C; sets Z, N, C.
    pub(crate) fn rol<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        let result = self.rol_value(bus.memory_read(addr));
        bus.memory_write(addr, result);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// ROR A (`$6A`): A ← (A >> 1) | C << 7, old bit 0 → C; sets Z, N, C.
    pub(crate) fn ror_accumulator<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.a = self.ror_value(self.a);
        self.pc = self.pc.wrapping_add(1);
    }

    /// ROR on memory: M ← (M >> 1) | C << 7, old bit 0 → C; sets Z, N, C.
    pub(crate) fn ror<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        let result = self.ror_value(bus.memory_read(addr));
        bus.memory_write(addr, result);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// Shift `value` left one bit: bit 7 to C, 0 into bit 0; sets Z, N, C.
    fn asl_value(&mut self, value: u8) -> u8 {
        self.set_flag(StatusFlag::Carry, value & 0x80 != 0);
        let result = value << 1;
        self.set_zero_and_negative_flags(result);
        result
    }

    /// Shift `value` right one bit: bit 0 to C, 0 into bit 7; sets Z, C, clears N.
    fn lsr_value(&mut self, value: u8) -> u8 {
        self.set_flag(StatusFlag::Carry, value & 0x01 != 0);
        let result = value >> 1;
        self.set_zero_and_negative_flags(result);
        result
    }

    /// Rotate `value` left through carry; sets Z, N, C. The incoming carry
    /// is sampled before the outgoing bit replaces it.
    #[rustfmt::skip]
    fn rol_value(&mut self, value: u8) -> u8 {
        let carry_in  = self.p & StatusFlag::Carry as u8 != 0;   // sample incoming C
        let carry_out = value & 0x80 != 0;                       // sample outgoing bit 7
        let result    = (value << 1) | u8::from(carry_in);
        self.set_flag(StatusFlag::Carry, carry_out);
        self.set_zero_and_negative_flags(result);
        result
    }

    /// Rotate `value` right through carry; sets Z, N, C. The incoming carry
    /// enters at bit 7.
    fn ror_value(&mut self, value: u8) -> u8 {
        let carry_in = self.flag(StatusFlag::Carry);
        let result = (value >> 1) | (u8::from(carry_in) << 7);
        self.set_flag(StatusFlag::Carry, value & 0x01 != 0);
        self.set_zero_and_negative_flags(result);
        result
    }
}
