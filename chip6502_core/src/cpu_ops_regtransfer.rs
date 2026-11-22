// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! Register transfers: TAX, TXA, TAY, TYA, TSX, TXS.
//!
//! Each copies one register into another without touching memory, and all
//! but `TXS` set Z and N from the copied value; `TXS` sets no flag, so
//! startup code can place the stack without disturbing the status register.
//! These are the clearest "twins" in the instruction set: each is the one
//! before it with a register name changed. Every handler takes the shared
//! `(mode, bus)` signature even though none needs either.
//!
//! Introduced in Build 7 — Interlude: A Filing System for the Match; the family is
//! completed in Build 22.

use crate::addressing::AddressingMode;
use crate::cpu::Chip6502;
use crate::Bus;

impl Chip6502 {
    /// TAX (`$AA`): X ← A; sets Z, N.
    pub(crate) fn tax<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.x = self.a;
        self.set_zero_and_negative_flags(self.x);
        self.pc = self.pc.wrapping_add(1);
    }

    /// TXA (`$8A`): A ← X; sets Z, N.
    pub(crate) fn txa<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.a = self.x;
        self.set_zero_and_negative_flags(self.a);
        self.pc = self.pc.wrapping_add(1);
    }

    /// TAY (`$A8`): Y ← A; sets Z, N.
    pub(crate) fn tay<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.y = self.a;
        self.set_zero_and_negative_flags(self.y);
        self.pc = self.pc.wrapping_add(1);
    }

    /// TYA (`$98`): A ← Y; sets Z, N.
    pub(crate) fn tya<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.a = self.y;
        self.set_zero_and_negative_flags(self.a);
        self.pc = self.pc.wrapping_add(1);
    }

    /// TSX (`$BA`): X ← SP; sets Z, N.
    pub(crate) fn tsx<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.x = self.sp;
        self.set_zero_and_negative_flags(self.x);
        self.pc = self.pc.wrapping_add(1);
    }

    /// TXS (`$9A`): SP ← X; no flags, alone among the transfers.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "one handler signature for all 61 handlers; `dispatch` calls them alike"
    )]
    pub(crate) fn txs<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.sp = self.x;
        self.pc = self.pc.wrapping_add(1);
    }
}
