// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! Arithmetic and comparison: ADC, SBC, CMP, CPX, CPY.
//!
//! All five run on the 6502's one adder. ADC adds the operand and the carry;
//! SBC adds the inverted operand and the carry, which is why its carry means
//! "no borrow"; the three compares subtract, keep the flags and discard the
//! result, and leave V alone. Each is one generic handler over its
//! addressing modes, and the arithmetic itself lives in operand-only
//! helpers (`adc_binary`, `sbc_binary`, `compare`) that need no bus.
//!
//! Introduced in Build 7 — Interlude: A Filing System for the Match;
//! decimal mode joins ADC and SBC in Build 25.

use crate::addressing::AddressingMode;
use crate::cpu::Chip6502;
use crate::{Bus, StatusFlag};

impl Chip6502 {
    /// ADC: A ← A + M + C, in binary or, with D set, packed BCD; sets N, V, Z, C.
    pub(crate) fn adc<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        let value = bus.memory_read(addr);
        if self.p & (StatusFlag::DecimalMode as u8) == 0 {
            self.adc_binary(value); // your existing logic, moved into a helper
        } else {
            self.adc_decimal(value);
        }
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// SBC: A ← A − M − (1 − C), in binary or, with D set, packed BCD;
    /// sets N, V, Z, C.
    pub(crate) fn sbc<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        let value = bus.memory_read(addr);
        if self.p & (StatusFlag::DecimalMode as u8) == 0 {
            self.sbc_binary(value);
        } else {
            self.sbc_decimal(value);
        }
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// Decimal-mode ADC, NMOS behavior (Build 25). The low nibble is
    /// corrected first, carrying exactly one ten into the high nibble even
    /// for invalid BCD; N and V come from the intermediate before the high
    /// correction and Z from the plain binary sum, as on the silicon. Only
    /// A and C reflect the decimal result.
    fn adc_decimal(&mut self, operand: u8) {
        let carry_in = u16::from(self.flag(StatusFlag::Carry));
        let a = u16::from(self.a);
        let m = u16::from(operand);

        let mut low = (a & 0x0F) + (m & 0x0F) + carry_in;
        if low >= 0x0A {
            low = ((low + 0x06) & 0x0F) + 0x10; // units digit, plus one ten carried
        }
        let mut high = (a & 0xF0) + (m & 0xF0) + low;

        // NMOS quirk: flags from the binary intermediates, before the high adjust.
        let binary_sum = a + m + carry_in;
        self.set_flag(StatusFlag::Zero, binary_sum.to_le_bytes()[0] == 0);
        self.set_flag(StatusFlag::Negative, high & 0x80 != 0);
        self.set_flag(StatusFlag::Overflow, (a ^ high) & (m ^ high) & 0x80 != 0);

        let carry_out = high >= 0xA0;
        if carry_out {
            high += 0x60;
        }
        self.set_flag(StatusFlag::Carry, carry_out);
        self.a = high.to_le_bytes()[0];
    }

    /// Decimal-mode SBC, NMOS behavior (Build 25). N, V, Z and C are the
    /// binary subtraction's, so the binary path sets them; A is then
    /// replaced by the nibble-corrected decimal result.
    fn sbc_decimal(&mut self, operand: u8) {
        let borrow = i16::from(!self.flag(StatusFlag::Carry));
        let a = i16::from(self.a);
        let m = i16::from(operand);

        self.sbc_binary(operand); // NMOS: the binary result's flags are the final flags

        let mut low = (a & 0x0F) - (m & 0x0F) - borrow;
        if low < 0 {
            low = ((low - 0x06) & 0x0F) - 0x10; // units digit, minus one ten borrowed
        }
        let mut high = (a & 0xF0) - (m & 0xF0) + low;
        if high < 0 {
            high -= 0x60;
        }
        self.a = high.to_le_bytes()[0];
    }

    /// CMP: flags from A − M; sets N, Z, C.
    pub(crate) fn cmp<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        let value = bus.memory_read(addr);
        self.compare(self.a, value);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// CPX: flags from X − M; sets N, Z, C.
    pub(crate) fn cpx<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        let value = bus.memory_read(addr);
        self.compare(self.x, value);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// CPY: flags from Y − M; sets N, Z, C.
    pub(crate) fn cpy<B: Bus>(&mut self, mode: AddressingMode, bus: &mut B) {
        let addr = self.resolve_addr(mode, bus);
        let value = bus.memory_read(addr);
        self.compare(self.y, value);
        self.pc = self.pc.wrapping_add(u16::from(mode.instruction_len()));
    }

    /// Binary add with carry: A ← A + operand + C; sets N, V, Z, C.
    /// V is set when both inputs share a sign and the result does not.
    fn adc_binary(&mut self, operand: u8) {
        let carry_in = self.p & StatusFlag::Carry as u8;
        let (partial, carry_1) = self.a.overflowing_add(operand);
        let (result, carry_2) = partial.overflowing_add(carry_in);
        let overflow = (self.a ^ result) & (operand ^ result) & 0x80 != 0;
        self.a = result;
        self.set_flag(StatusFlag::Carry, carry_1 || carry_2);
        self.set_flag(StatusFlag::Overflow, overflow);
        self.set_zero_and_negative_flags(self.a);
    }

    /// Binary subtract with borrow, A ← A − operand − (1 − C), done the way
    /// the silicon does it: the same adder, fed the inverted operand
    /// (Build 8). Sets N, V, Z, C, with C meaning "no borrow".
    fn sbc_binary(&mut self, operand: u8) {
        self.adc_binary(!operand);
    }

    /// The shared body of CMP, CPX and CPY: `register − operand`, keeping
    /// only the flags. Sets C (no borrow), Z and N; leaves V alone.
    fn compare(&mut self, register: u8, operand: u8) {
        self.set_flag(StatusFlag::Carry, register >= operand);
        self.set_zero_and_negative_flags(register.wrapping_sub(operand));
    }
}
