// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! System instructions and interrupt entry: BRK, NOP, RTI, IRQ, NMI.
//!
//! IRQ, NMI and `BRK` share one entry sequence (push PC and status, set I,
//! jump through a vector) and differ only in the vector, in the B bit of the
//! pushed status, which is how a handler tells `BRK` from a hardware IRQ,
//! and in `BRK`'s extra step past its marker byte. `RTI` pulls that frame
//! back with no `+1`. Every opcode `dispatch` does not route to a handler
//! arrives at `not_implemented`, where this crate decides what an unmodeled
//! opcode means.
//!
//! Introduced in Build 22 — Interlude: Completing the Instruction Set;
//! Build 23 adds interrupt entry and Build 28 settles `not_implemented`.

use crate::addressing::AddressingMode;
use crate::cpu::Chip6502;
use crate::{Bus, StatusFlag};

/// Where the NMI handler's address lives.
const NMI_VECTOR: u16 = 0xFFFA;
/// Where the IRQ and BRK handler's address lives; the two share it.
const IRQ_BRK_VECTOR: u16 = 0xFFFE;

impl Chip6502 {
    /// BRK (`$00`): push PC + 2 (skipping the marker byte after the opcode),
    /// push P with B and U set, set I, and jump through `$FFFE/$FFFF`.
    pub(crate) fn brk<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        self.pc = self.pc.wrapping_add(2);
        self.enter_interrupt(IRQ_BRK_VECTOR, true, bus);
    }

    /// Take an IRQ: push PC and P (B clear), set I, jump through `$FFFE/$FFFF`.
    /// The IRQ line stays asserted; the host releases it.
    pub(crate) fn take_irq<B: Bus>(&mut self, bus: &mut B) {
        self.enter_interrupt(IRQ_BRK_VECTOR, false, bus);
    }

    /// Take an NMI: push PC and P (B clear), set I, jump through
    /// `$FFFA/$FFFB`, and clear the pending edge.
    pub(crate) fn take_nmi<B: Bus>(&mut self, bus: &mut B) {
        self.nmi_pending = false;
        self.enter_interrupt(NMI_VECTOR, false, bus);
    }

    /// The entry sequence IRQ, NMI and BRK share: push PCH, PCL, then P with
    /// U set and B set only for BRK; set I; load PC from `vector`. D is left
    /// alone, as on the NMOS part.
    fn enter_interrupt<B: Bus>(&mut self, vector: u16, brk: bool, bus: &mut B) {
        self.stack_push_u16(self.pc, bus);
        let pushed_p = if brk {
            self.p | StatusFlag::Unused as u8 | StatusFlag::BreakCommand as u8
        } else {
            (self.p | StatusFlag::Unused as u8) & !(StatusFlag::BreakCommand as u8)
        };
        self.stack_push(pushed_p, bus);
        self.set_flag(StatusFlag::InterruptDisable, true);
        let lo = bus.memory_read(vector);
        let hi = bus.memory_read(vector.wrapping_add(1));
        self.pc = u16::from_le_bytes([lo, hi]);
    }

    /// NOP (`$EA`): advance PC; nothing else changes.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "one handler signature for all 61 handlers; `dispatch` calls them alike"
    )]
    pub(crate) fn nop<B: Bus>(&mut self, _mode: AddressingMode, _bus: &mut B) {
        self.pc = self.pc.wrapping_add(1);
    }

    /// RTI (`$40`): pull P (B masked off, U forced on), then PC low and high.
    /// No `+1`, unlike RTS.
    pub(crate) fn rti<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        let pulled = self.stack_pop(bus);
        self.p = (pulled & !(StatusFlag::BreakCommand as u8)) | StatusFlag::Unused as u8;
        self.pc = self.stack_pop_u16(bus);
    }

    /// The catch-all for opcodes with no handler (Build 28's failure model).
    ///
    /// In this crate's own test builds, reaching it is a gap in our table,
    /// so it panics. In a normal build it is an emulated program running an
    /// opcode the core does not model: the core records the opcode and PC
    /// for [`Chip6502::last_unimplemented_opcode`] and does not advance PC.
    // Audit verdict (Build 28): keep the cfg split; the production arm
    // records instead of printing.
    #[cfg_attr(
        test,
        expect(
            clippy::needless_pass_by_ref_mut,
            reason = "only the normal-build arm writes to self"
        )
    )]
    pub(crate) fn not_implemented<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        // No `op` parameter — every handler shares the OPCODES table's
        // uniform (mode, bus) signature. But PC still points at the
        // opcode byte while a handler runs, so one read recovers it.
        // Declared above both cfg arms so each surviving arm uses it.
        let op = bus.memory_read(self.pc);

        // Test builds: a gap in OUR opcode table is a programmer error.
        #[cfg(test)]
        panic!(
            "opcode {op:#04X} at PC {:#06X} routed to not_implemented — \
             handler exists but no OPCODES row, or opcode unimplemented",
            self.pc
        );

        // Production: the same event is DATA — an emulated program ran
        // an opcode we don't model. Keep running (the no-panic rule),
        // but record the fact as inspectable state so the embedder or
        // debugger can surface it. The core prints nothing, ever.
        #[cfg(not(test))]
        {
            self.last_unimplemented_opcode = Some((op, self.pc));
        }
    }
}
