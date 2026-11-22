// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! The thirteen 6502 addressing modes, and the instruction length each implies.
//!
//! An opcode is a mnemonic plus an addressing mode, and the mode, not the
//! mnemonic, decides how many bytes the instruction occupies; so the length
//! is a `const`, total function of the mode. All thirteen variants are
//! documented. The enum is `pub` even though its module
//! is private, so that `dispatch` and `OpcodeInfo` can be re-exported without
//! tripping `private_interfaces`.
//!
//! Introduced in Build 3 — Interlude: The disassembler; the `OPCODES` table and
//! the generic handlers take it as a value in Build 22.

/// One of the thirteen 6502 addressing modes.
#[derive(Copy, Clone, Debug)]
#[rustfmt::skip]
pub enum AddressingMode {
    /// No operand: the instruction names what it works on (`INX`, `RTS`). (1 byte)
    Implied,
    /// Operates on A directly (`ASL A`). (1 byte)
    Accumulator,     
    /// The operand is the literal byte after the opcode (`LDA #$42`). (2 bytes)
    Immediate,       
    /// A one-byte address in page zero (`LDA $42`). (2 bytes)
    ZeroPage,        
    /// Zero page plus X, wrapping within page zero (`LDA $42,X`). (2 bytes)
    ZeroPageX,       
    /// Zero page plus Y, wrapping within page zero; LDX and STX only. (2 bytes)
    ZeroPageY,       
    /// A signed offset from the next instruction; branches only. (2 bytes)
    Relative,        
    /// A full 16-bit address, low byte first (`LDA $1234`). (3 bytes)
    Absolute,        
    /// An absolute address plus X (`LDA $1234,X`). (3 bytes)
    AbsoluteX,       
    /// An absolute address plus Y (`LDA $1234,Y`). (3 bytes)
    AbsoluteY,       
    /// A pointer to the target; `JMP ($1234)` only, with the page-boundary bug. (3 bytes)
    Indirect,        
    /// `(zp,X)`: add X to the zero-page operand, then read the pointer there. (2 bytes)
    IndexedIndirect, 
    /// `(zp),Y`: read the pointer at the zero-page operand, then add Y. (2 bytes)
    IndirectIndexed, 
}

impl AddressingMode {
    /// Total instruction length in bytes (opcode + operand) for this mode.
    #[rustfmt::skip]
    pub(crate) const fn instruction_len(self) -> u8 {
        match self {
            Self::Implied | Self::Accumulator => 1,
            Self::Immediate
            | Self::ZeroPage  | Self::ZeroPageX | Self::ZeroPageY
            | Self::Relative
            | Self::IndexedIndirect | Self::IndirectIndexed => 2,
            Self::Absolute | Self::AbsoluteX | Self::AbsoluteY
            | Self::Indirect => 3,
        }
    }
}
