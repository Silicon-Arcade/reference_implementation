// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! Increments and decrements: INC, DEC, INX, DEX, INY, DEY.
//!
//! Each adds or subtracts one with 8-bit wraparound and sets Z and N from
//! the result. None of them touches C, so they cannot carry from one byte
//! to the next the way ADC does; a multi-byte increment tests Z instead.
//! The memory forms are generic read-modify-write handlers that do a single
//! write: on plain RAM the NMOS dummy write of the old value cannot be seen
//! (Build 6 records the choice).
//!
//! Introduced in Build 7 — Interlude: A Filing System for the Match; the generic memory forms arrive in Build 22.

use crate::addressing::AddressingMode;
use crate::cpu::Chip6502;
use crate::Bus;

impl Chip6502 {
    /// INC: M ← M + 1; sets Z, N.
    pub(crate) fn inc<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        let value = bus.memory_read(addr).wrapping_add(1);
        bus.memory_write(addr, value);
        self.set_zero_and_negative_flags(value);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// DEC: M ← M − 1; sets Z, N.
    pub(crate) fn dec<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        let value = bus.memory_read(addr).wrapping_sub(1);
        bus.memory_write(addr, value);
        self.set_zero_and_negative_flags(value);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// INX (`$E8`): X ← X + 1; sets Z, N.
    pub(crate) fn inx<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.x = self.x.wrapping_add(1);
        self.set_zero_and_negative_flags(self.x);
        self.pc = self.pc.wrapping_add(1);
    }

    /// DEX (`$CA`): X ← X − 1; sets Z, N.
    pub(crate) fn dex<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.x = self.x.wrapping_sub(1);
        self.set_zero_and_negative_flags(self.x);
        self.pc = self.pc.wrapping_add(1);
    }

    /// INY (`$C8`): Y ← Y + 1; sets Z, N.
    pub(crate) fn iny<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.y = self.y.wrapping_add(1);
        self.set_zero_and_negative_flags(self.y);
        self.pc = self.pc.wrapping_add(1);
    }

    /// DEY (`$88`): Y ← Y − 1; sets Z, N.
    pub(crate) fn dey<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.y = self.y.wrapping_sub(1);
        self.set_zero_and_negative_flags(self.y);
        self.pc = self.pc.wrapping_add(1);
    }
}
