// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! Jumps and calls: JMP, JSR, RTS.
//!
//! These set PC rather than advance it, so their handlers end without the
//! usual length step. `JSR` pushes the address of its own last byte, one
//! less than the return address, and `RTS` adds the one back; programs
//! depend on that `+1`, so it is modeled exactly. `JMP (indirect)` carries
//! the NMOS page-boundary bug: a pointer at `$xxFF` takes its high byte from `$xx00`.
//!
//! Introduced in Build 7 — Interlude: A Filing System for the
//! Match; JSR and RTS first ran in Build 2, `JMP (indirect)` in Build 11.

use crate::addressing::AddressingMode;
use crate::cpu::Chip6502;
use crate::Bus;

impl Chip6502 {
    /// JMP abs (`$4C`): PC ← address; no flags.
    pub(crate) fn jmp_absolute<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        self.pc = self.resolve_addr_absolute(bus);
    }

    /// JMP (ind) (`$6C`): PC ← the word at the pointer, with the NMOS
    /// page-boundary bug: a pointer at `$xxFF` reads its high byte from `$xx00`.
    #[rustfmt::skip]
    pub(crate) fn jmp_indirect<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {   // JMP (indirect), $6C
        let low  = u16::from(bus.memory_read(self.pc.wrapping_add(1)));
        let high = u16::from(bus.memory_read(self.pc.wrapping_add(2)));
        let ptr = (high << 8) | low;

        // Low byte of target: read normally
        let lo = bus.memory_read(ptr);

        // High byte of target: BUG — wraps within the same page
        let hi_addr = (ptr & 0xFF00) | (ptr.wrapping_add(1) & 0x00FF);
        let hi = bus.memory_read(hi_addr);

        self.pc = u16::from_le_bytes([lo, hi]);
        // no length advance — JMP's job is to set PC
    }

    /// JSR abs (`$20`): push PC + 2 (the JSR's own last byte), PC ← address.
    pub(crate) fn jsr_absolute<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        let target = self.resolve_addr_absolute(bus);
        let return_addr = self.pc.wrapping_add(2); // the JSR's own last byte
        self.stack_push_u16(return_addr, bus);
        self.pc = target;
    }

    /// RTS (`$60`): PC ← pulled address + 1; no flags.
    pub(crate) fn rts<B: Bus>(&mut self, _mode: AddressingMode, bus: &mut B) {
        self.pc = self.stack_pop_u16(bus).wrapping_add(1);
    }
}
