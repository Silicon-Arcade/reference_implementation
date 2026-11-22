// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! Load and store: LDA, LDX, LDY, STA, STX, STY.
//!
//! These six mnemonics move bytes between memory and the registers and
//! account for 31 of the 151 opcodes, because each accepts many addressing
//! modes. Each is one generic handler: `resolve_addr` supplies the effective
//! address for whichever mode the `OPCODES` row names, and the handler ends
//! by advancing PC by that mode's length. Loads set Z and N from the value;
//! stores never touch a flag.
//!
//! Introduced in Build 7 — Interlude: A Filing System for the Match; collapsed to one handler per mnemonic in Build 22.

use crate::addressing::AddressingMode;
use crate::cpu::Chip6502;
use crate::Bus;

impl Chip6502 {
    /// LDA: A ← M; sets Z, N.
    #[rustfmt::skip]
    pub(crate) fn lda<B: Bus>(&mut self, addressing_mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(addressing_mode, bus);
        let value = bus.memory_read(addr);
        self.set_zero_and_negative_flags(value);
        self.a = value;
        self.pc = self.pc.wrapping_add(u16::from(addressing_mode.instruction_len()));
    }

    /// LDX: X ← M; sets Z, N.
    pub(crate) fn ldx<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        self.x = bus.memory_read(addr);
        self.set_zero_and_negative_flags(self.x);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// LDY: Y ← M; sets Z, N.
    pub(crate) fn ldy<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        self.y = bus.memory_read(addr);
        self.set_zero_and_negative_flags(self.y);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// STA: M ← A; no flags.
    pub(crate) fn sta<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        bus.memory_write(addr, self.a);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// STX: M ← X; no flags.
    pub(crate) fn stx<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        bus.memory_write(addr, self.x);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// STY: M ← Y; no flags.
    pub(crate) fn sty<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        bus.memory_write(addr, self.y);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }
}
