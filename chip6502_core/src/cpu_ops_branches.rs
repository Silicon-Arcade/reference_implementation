// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! Conditional branches: BCC, BCS, BEQ, BNE, BMI, BPL, BVC, BVS.
//!
//! Each tests one flag and, if the condition holds, adds a signed offset to
//! the address of the next instruction; none of them changes a flag. The
//! offset byte is reinterpreted as `i8` and then sign-extended to `u16`, in
//! that order, so `wrapping_add` turns a negative offset into a subtraction.
//! A branch not taken simply steps past its two bytes.
//!
//! Introduced in Build 7 — Interlude: A Filing System for the Match; the signed-offset
//! rule is Build 1's; `branch_if` is the body all eight share, and from Build 27
//! it also charges the taken-branch and page-crossing cycles.

use crate::addressing::AddressingMode;
use crate::cpu::Chip6502;
use crate::{Bus, StatusFlag};

impl Chip6502 {
    /// BCC (`$90`): branch if C = 0.
    pub(crate) fn bcc_relative<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        self.branch_if(!self.flag(StatusFlag::Carry), bus);
    }

    /// BCS (`$B0`): branch if C = 1.
    pub(crate) fn bcs_relative<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        self.branch_if(self.flag(StatusFlag::Carry), bus);
    }

    /// BEQ (`$F0`): branch if Z = 1.
    pub(crate) fn beq_relative<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        self.branch_if(self.flag(StatusFlag::Zero), bus);
    }

    /// BNE (`$D0`): branch if Z = 0.
    pub(crate) fn bne_relative<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        self.branch_if(!self.flag(StatusFlag::Zero), bus);
    }

    /// BMI (`$30`): branch if N = 1.
    pub(crate) fn bmi_relative<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        self.branch_if(self.flag(StatusFlag::Negative), bus);
    }

    /// BPL (`$10`): branch if N = 0.
    pub(crate) fn bpl_relative<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        self.branch_if(!self.flag(StatusFlag::Negative), bus);
    }

    /// BVC (`$50`): branch if V = 0.
    pub(crate) fn bvc_relative<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        self.branch_if(!self.flag(StatusFlag::Overflow), bus);
    }

    /// BVS (`$70`): branch if V = 1.
    pub(crate) fn bvs_relative<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        self.branch_if(self.flag(StatusFlag::Overflow), bus);
    }

    /// The body every branch shares: step past the two bytes, then, if
    /// `condition`, add the signed offset, charging 1 cycle for the taken
    /// branch and 1 more if the target is on a different page from the
    /// instruction after the branch.
    #[expect(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "Build 1's two branch casts reinterpret the offset as signed, then sign-extend it"
    )]
    #[rustfmt::skip]
    fn branch_if<B: Bus>(&mut self, condition: bool, bus: &mut B) {
        let offset = bus.memory_read(self.pc.wrapping_add(1)) as i8;   // reinterpret the byte as signed
        self.pc = self.pc.wrapping_add(2);                             // advance past the branch
        if condition {
            let target = self.pc.wrapping_add(offset as u16);          // sign-extend, add, wrap
            self.last_step_cycles += 1;                                // branch taken
            if (self.pc & 0xFF00) != (target & 0xFF00) {
                self.last_step_cycles += 1;                            // crossed a page
            }
            self.pc = target;
        }
    }
}
