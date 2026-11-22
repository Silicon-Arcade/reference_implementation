// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! Machinery the handlers share: the hardware stack and the address resolvers.
//!
//! The 6502 stack is 256 bytes of ordinary RAM at `$0100`–`$01FF`, addressed
//! through the 8-bit SP, so it lives in emulated memory rather than in a
//! host-side `Vec`: real programs read and rewrite pushed bytes directly.
//!
//! `resolve_addr` turns an addressing mode into an effective address, one
//! small resolver per addressing mode, so a generic handler need not care which of its
//! modes it is serving; immediate mode's "address" is simply the byte after
//! the opcode. The zero-page forms wrap in `u8` arithmetic before widening,
//! so they can never leave page zero.
//!
//! Introduced in Build 2 — Count 0 to 9 with subroutines (the stack); the resolvers arrive in Build 22.

use crate::addressing::AddressingMode;
// `allow`, not `expect`: clippy reports this lint for the library build but
// not for the test build, so an expectation is unfulfilled in one of them.
#[allow(
    clippy::enum_glob_use,
    reason = "Build 22 prints the glob import so `resolve_addr`'s arms can name bare variants"
)]
use crate::addressing::AddressingMode::*;
use crate::{Bus, Chip6502};

impl Chip6502 {
    /// Push a byte to the stack at `$0100 + SP`, then decrement SP with
    /// 8-bit wrap.
    pub(crate) fn stack_push<B: Bus>(&mut self, value: u8, bus: &mut B) {
        let addr = 0x0100 | u16::from(self.sp);
        bus.memory_write(addr, value);
        self.sp = self.sp.wrapping_sub(1);
    }

    /// Increment SP with 8-bit wrap, then read the byte at `$0100 + SP`.
    pub(crate) fn stack_pop<B: Bus>(&mut self, bus: &mut B) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        let addr = 0x0100 | u16::from(self.sp);
        bus.memory_read(addr)
    }

    /// Push a 16-bit value high-byte-first (so the low byte ends up at
    /// the lower stack address — matches JSR's push order).
    pub(crate) fn stack_push_u16<B: Bus>(&mut self, value: u16, bus: &mut B) {
        self.stack_push((value >> 8) as u8, bus);
        self.stack_push((value & 0xFF) as u8, bus);
    }

    /// Pop a 16-bit value low-byte-first (matches RTS/RTI's pop order).
    pub(crate) fn stack_pop_u16<B: Bus>(&mut self, bus: &mut B) -> u16 {
        let low = u16::from(self.stack_pop(bus));
        let high = u16::from(self.stack_pop(bus));
        (high << 8) | low
    }

    /// The effective address of the operand for `mode`, read relative to
    /// the opcode at PC.
    ///
    /// # Panics
    ///
    /// Panics if `mode` has no resolver (`Implied`, `Accumulator`,
    /// `Indirect`): that means an `OPCODES` row gives a generic handler a
    /// mode it cannot serve, a bug in this crate's table.
    #[rustfmt::skip]
    pub(crate) fn resolve_addr<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) -> u16 {
        match mode {
            Immediate => self.resolve_addr_immediate(bus),
            ZeroPage => self.resolve_addr_zero_page(bus),
            ZeroPageX => self.resolve_addr_zero_page_x(bus),
            ZeroPageY => self.resolve_addr_zero_page_y(bus),
            Absolute => self.resolve_addr_absolute(bus),
            AbsoluteX => self.resolve_addr_absolute_x(bus),
            AbsoluteY => self.resolve_addr_absolute_y(bus),
            IndexedIndirect => self.resolve_addr_indexed_indirect(bus),
            IndirectIndexed => self.resolve_addr_indirect_indexed(bus),
            Relative => self.resolve_addr_relative(bus),
            // Audit verdict (Build 28): keep — a contract violation inside this
            // crate; the message names the invariant, the evidence and the fix.
            _ => panic!(
                "OPCODES table bug: opcode {:#04X} at PC {:#06X} claims addressing \
                mode {mode:?}, but resolve_addr has no arm for it — fix the OPCODES \
                row or add the resolver",
                bus.memory_read(self.pc), self.pc
            ),
        }
    }

    /// Immediate: the operand's "address" is the byte after the opcode.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "kept as the plain `fn` the book prints in Build 22"
    )]
    pub(crate) fn resolve_addr_immediate<B: Bus>(&self, _bus: &mut B) -> u16 {
        self.pc.wrapping_add(1)
    }

    /// Zero page: `$00LL`.
    pub(crate) fn resolve_addr_zero_page<B: Bus>(&self, bus: &mut B) -> u16 {
        u16::from(bus.memory_read(self.pc.wrapping_add(1)))
    }

    /// Zero page,X: `($LL + X) & $FF`, wrapping within page zero.
    #[expect(
        clippy::needless_pass_by_ref_mut,
        reason = "Build 22 prints this resolver taking `&mut self`"
    )]
    pub(crate) fn resolve_addr_zero_page_x<B: Bus>(&mut self, bus: &mut B) -> u16 {
        let zp = bus.memory_read(self.pc.wrapping_add(1));
        u16::from(zp.wrapping_add(self.x)) // wraps in zero page (u8 arithmetic)
    }

    /// Zero page,Y: `($LL + Y) & $FF`, wrapping within page zero.
    pub(crate) fn resolve_addr_zero_page_y<B: Bus>(&self, bus: &mut B) -> u16 {
        let zp = bus.memory_read(self.pc.wrapping_add(1));
        u16::from(zp.wrapping_add(self.y))
    }

    /// Absolute: the two operand bytes, low byte first.
    pub(crate) fn resolve_addr_absolute<B: Bus>(&self, bus: &mut B) -> u16 {
        let lo = bus.memory_read(self.pc.wrapping_add(1));
        let hi = bus.memory_read(self.pc.wrapping_add(2));
        u16::from_le_bytes([lo, hi])
    }

    /// Absolute,X: base + X as a full 16-bit add; records a page crossing.
    pub(crate) fn resolve_addr_absolute_x<B: Bus>(&mut self, bus: &mut B) -> u16 {
        let base_addr = self.resolve_addr_absolute(bus);
        let effective_addr = base_addr.wrapping_add(u16::from(self.x));
        // Detect only — whether this costs a cycle is the OPCODES
        // table's decision, not ours (STA abs,X never pays).
        self.page_crossed = (base_addr & 0xFF00) != (effective_addr & 0xFF00);
        effective_addr
    }

    /// Absolute,Y: base + Y as a full 16-bit add; records a page crossing.
    pub(crate) fn resolve_addr_absolute_y<B: Bus>(&mut self, bus: &mut B) -> u16 {
        let base_addr = self.resolve_addr_absolute(bus);
        let effective_addr = base_addr.wrapping_add(u16::from(self.y));
        // Detect only — whether this costs a cycle is the OPCODES
        // table's decision, not ours (STA abs,Y never pays).
        self.page_crossed = (base_addr & 0xFF00) != (effective_addr & 0xFF00);
        effective_addr
    }

    /// `(zp,X)`: add X to the zero-page operand (wrapping), then read the
    /// pointer there, its high byte also wrapping within page zero.
    pub(crate) fn resolve_addr_indexed_indirect<B: Bus>(&self, bus: &mut B) -> u16 {
        let zp = bus
            .memory_read(self.pc.wrapping_add(1))
            .wrapping_add(self.x);
        let lo = bus.memory_read(u16::from(zp));
        let hi = bus.memory_read(u16::from(zp.wrapping_add(1)));
        u16::from_le_bytes([lo, hi])
    }

    /// `(zp),Y`: read a pointer from zero page, wrapping the high-byte fetch
    /// within page zero, then add Y as a full 16-bit add; records a page
    /// crossing on that final add.
    pub(crate) fn resolve_addr_indirect_indexed<B: Bus>(&mut self, bus: &mut B) -> u16 {
        let zp = bus.memory_read(self.pc.wrapping_add(1));
        let lo = bus.memory_read(u16::from(zp));
        let hi = bus.memory_read(u16::from(zp.wrapping_add(1)));
        let base_addr = u16::from_le_bytes([lo, hi]);
        let effective_addr = base_addr.wrapping_add(u16::from(self.y));
        self.page_crossed = (base_addr & 0xFF00) != (effective_addr & 0xFF00);
        effective_addr
    }

    /// Relative: the branch target, the signed offset added to the address
    /// of the next instruction.
    #[expect(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "Build 1's two branch casts reinterpret the offset as signed, then sign-extend it"
    )]
    pub(crate) fn resolve_addr_relative<B: Bus>(&self, bus: &mut B) -> u16 {
        let offset = bus.memory_read(self.pc.wrapping_add(1)) as i8;
        self.pc.wrapping_add(2).wrapping_add(offset as u16)
    }
}
