// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! The eight bits of the 6502 status register, named once.
//!
//! Each variant's discriminant is the bit's mask, so `StatusFlag::Carry as
//! u8` is `$01` and is ready to AND or OR with `p`. All eight are named on
//! the day the first flag is needed, so no later build has to stop and
//! define one. B (`BreakCommand`) and U (`Unused`) are not storage in the
//! processor; they matter only in status bytes pushed on the stack, which
//! Build 23 explains.
//!
//! Introduced in Chapter 5 — The Method and the Map, in the Opcode Development Loop walkthrough.

/// One status-register bit, encoded as the bitmask used to set / clear it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum StatusFlag {
    /// C, bit 0: the carry out of an add, or "no borrow" after a subtract.
    Carry = 0b0000_0001,
    /// Z, bit 1: the result was zero.
    Zero = 0b0000_0010,
    /// I, bit 2: when set, IRQs are masked.
    InterruptDisable = 0b0000_0100,
    /// D, bit 3: when set, `ADC` and `SBC` use binary-coded decimal.
    DecimalMode = 0b0000_1000,
    /// B, bit 4: set in a status byte pushed by `BRK` or `PHP`, clear in one
    /// pushed by IRQ or NMI. Not stored in the live register.
    BreakCommand = 0b0001_0000,
    /// U, bit 5: reads as 1 in every pushed status byte.
    Unused = 0b0010_0000,
    /// V, bit 6: signed overflow from `ADC`/`SBC`, or bit 6 of `BIT`'s operand.
    Overflow = 0b0100_0000,
    /// N, bit 7: a copy of bit 7 of the result.
    Negative = 0b1000_0000,
}
