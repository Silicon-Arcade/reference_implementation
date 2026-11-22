// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! Flag operations: CLC, SEC, CLD, SED, CLI, SEI, CLV.
//!
//! Each sets or clears exactly one status bit and leaves the other seven
//! alone; there is no SEV, because only arithmetic and `BIT` set V. Note the
//! inversion in the interrupt pair: `CLI` *enables* IRQs by clearing the
//! disable bit. The carry pair matter most, since `ADC` and `SBC` always
//! read C.
//!
//! Introduced in Build 7 — Interlude: A Filing System for the Match; the family is completed in Build 22.

use crate::addressing::AddressingMode;
use crate::cpu::Chip6502;
use crate::{Bus, StatusFlag};

impl Chip6502 {
    /// CLC (`$18`): C ← 0.
    pub(crate) fn clc<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.set_flag(StatusFlag::Carry, false);
        self.pc = self.pc.wrapping_add(1);
    }

    /// SEC (`$38`): C ← 1.
    pub(crate) fn sec<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.set_flag(StatusFlag::Carry, true);
        self.pc = self.pc.wrapping_add(1);
    }

    /// CLD (`$D8`): D ← 0; ADC and SBC return to binary.
    pub(crate) fn cld<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.set_flag(StatusFlag::DecimalMode, false);
        self.pc = self.pc.wrapping_add(1);
    }

    /// SED (`$F8`): D ← 1; ADC and SBC use packed BCD.
    pub(crate) fn sed<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.set_flag(StatusFlag::DecimalMode, true);
        self.pc = self.pc.wrapping_add(1);
    }

    /// CLI (`$58`): I ← 0, which *enables* IRQs.
    pub(crate) fn cli<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.set_flag(StatusFlag::InterruptDisable, false);
        self.pc = self.pc.wrapping_add(1);
    }

    /// SEI (`$78`): I ← 1, which masks IRQs (NMI is unaffected).
    pub(crate) fn sei<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.set_flag(StatusFlag::InterruptDisable, true);
        self.pc = self.pc.wrapping_add(1);
    }

    /// CLV (`$B8`): V ← 0.
    pub(crate) fn clv<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.set_flag(StatusFlag::Overflow, false);
        self.pc = self.pc.wrapping_add(1);
    }
}
