// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! Stack instructions: PHA, PLA, PHP, PLP.
//!
//! Each pushes or pulls one byte through the Build 2 stack helpers, so the
//! bytes land in page `$01` of emulated RAM where a program can see them.
//! PLA is a load and sets Z and N; PHA sets nothing. B and U exist only in
//! pushed status bytes: PHP pushes them both set, and PLP masks B off and
//! forces U on, so a pulled byte can never plant a B the live register does
//! not have.
//!
//! Introduced in Build 7 — Interlude: A Filing System for the
//! Match; PHP and PLP arrive in Build 22, and Appendix B gives the rules.

use crate::addressing::AddressingMode;
use crate::cpu::Chip6502;
use crate::{Bus, StatusFlag};

impl Chip6502 {
    /// PHA (`$48`): push A; no flags.
    pub(crate) fn pha<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        self.stack_push(self.a, bus);
        self.pc = self.pc.wrapping_add(1);
    }

    /// PLA (`$68`): A ← pulled byte; sets Z, N.
    pub(crate) fn pla<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        self.a = self.stack_pop(bus);
        self.set_zero_and_negative_flags(self.a);
        self.pc = self.pc.wrapping_add(1);
    }

    /// PHP (`$08`): push P with B and U set in the pushed copy; no flags.
    pub(crate) fn php<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        let pushed_p = self.p | StatusFlag::Unused as u8 | StatusFlag::BreakCommand as u8;
        self.stack_push(pushed_p, bus);
        self.pc = self.pc.wrapping_add(1);
    }

    /// PLP (`$28`): P ← pulled byte with B masked off and U forced on.
    pub(crate) fn plp<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        let pulled = self.stack_pop(bus);
        self.p = (pulled & !(StatusFlag::BreakCommand as u8)) | StatusFlag::Unused as u8;
        self.pc = self.pc.wrapping_add(1);
    }
}
