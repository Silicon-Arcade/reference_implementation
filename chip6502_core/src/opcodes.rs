// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! The opcode table and the dispatch: facts as data, behavior as code.
//!
//! `OPCODES` holds one row of facts per opcode byte (mnemonic, addressing
//! mode, base cycles, and whether a page crossing costs a cycle), and
//! `dispatch` routes each opcode to the handler for its *instruction*,
//! passing the mode from the row. The table holds no function pointers:
//! every handler is generic over the bus type, and a Rust `const` cannot
//! easily hold function pointers generic over a `Bus` implementation, so a
//! grouped `match` does the routing and the compiler turns it into a jump
//! table. The table is written one visible row per opcode, because a table
//! you can read is a table you can audit against Appendix C.
//!
//! Introduced in Build 22 — Interlude: Completing the Instruction Set;
//! Build 27 reads the `cycles` and `page_sensitive` columns.

use crate::addressing::AddressingMode;
use crate::cpu::Chip6502;
use crate::Bus;

/// Per-opcode static metadata. The dispatch handler is *not* stored
/// here — see [`dispatch`].
#[derive(Copy, Clone)]
#[rustfmt::skip]
pub struct OpcodeInfo {
    /// The instruction's three-letter name, or `"???"` for an opcode this
    /// core does not model.
    pub mnemonic: &'static str,
    /// How the instruction forms its operand's address.
    pub addressing_mode: AddressingMode,
    /// Base cycles, before page-cross penalties.
    pub cycles: u8,                 // Base cycles, before page-cross penalties
    /// True if the instruction is sensitive to page crossings.
    pub page_sensitive: bool,       // True if the instruction is sensitive to page crossings
}

/// One row per opcode byte: the 151 documented opcodes, and a visible
/// `"???"` placeholder in the other 105 slots.
#[rustfmt::skip]
pub const OPCODES: [OpcodeInfo; 256] = {
    let mut table = [OpcodeInfo { mnemonic: "???", addressing_mode: AddressingMode::Implied, cycles: 0, page_sensitive: false }; 256];

    // ----- Load / Store -----
    table[0xA9] = OpcodeInfo { mnemonic: "LDA", addressing_mode: AddressingMode::Immediate,       cycles: 2, page_sensitive: false };
    table[0xA5] = OpcodeInfo { mnemonic: "LDA", addressing_mode: AddressingMode::ZeroPage,        cycles: 3, page_sensitive: false };
    table[0xB5] = OpcodeInfo { mnemonic: "LDA", addressing_mode: AddressingMode::ZeroPageX,       cycles: 4, page_sensitive: false };
    table[0xAD] = OpcodeInfo { mnemonic: "LDA", addressing_mode: AddressingMode::Absolute,        cycles: 4, page_sensitive: false };
    table[0xBD] = OpcodeInfo { mnemonic: "LDA", addressing_mode: AddressingMode::AbsoluteX,       cycles: 4, page_sensitive: true };
    table[0xB9] = OpcodeInfo { mnemonic: "LDA", addressing_mode: AddressingMode::AbsoluteY,       cycles: 4, page_sensitive: true };
    table[0xA1] = OpcodeInfo { mnemonic: "LDA", addressing_mode: AddressingMode::IndexedIndirect, cycles: 6, page_sensitive: false };
    table[0xB1] = OpcodeInfo { mnemonic: "LDA", addressing_mode: AddressingMode::IndirectIndexed, cycles: 5, page_sensitive: true };

    table[0xA2] = OpcodeInfo { mnemonic: "LDX", addressing_mode: AddressingMode::Immediate,       cycles: 2, page_sensitive: false };
    table[0xA6] = OpcodeInfo { mnemonic: "LDX", addressing_mode: AddressingMode::ZeroPage,        cycles: 3, page_sensitive: false };
    table[0xB6] = OpcodeInfo { mnemonic: "LDX", addressing_mode: AddressingMode::ZeroPageY,       cycles: 4, page_sensitive: false };
    table[0xAE] = OpcodeInfo { mnemonic: "LDX", addressing_mode: AddressingMode::Absolute,        cycles: 4, page_sensitive: false };
    table[0xBE] = OpcodeInfo { mnemonic: "LDX", addressing_mode: AddressingMode::AbsoluteY,       cycles: 4, page_sensitive: true };

    table[0xA0] = OpcodeInfo { mnemonic: "LDY", addressing_mode: AddressingMode::Immediate,       cycles: 2, page_sensitive: false };
    table[0xA4] = OpcodeInfo { mnemonic: "LDY", addressing_mode: AddressingMode::ZeroPage,        cycles: 3, page_sensitive: false };
    table[0xB4] = OpcodeInfo { mnemonic: "LDY", addressing_mode: AddressingMode::ZeroPageX,       cycles: 4, page_sensitive: false };
    table[0xAC] = OpcodeInfo { mnemonic: "LDY", addressing_mode: AddressingMode::Absolute,        cycles: 4, page_sensitive: false };
    table[0xBC] = OpcodeInfo { mnemonic: "LDY", addressing_mode: AddressingMode::AbsoluteX,       cycles: 4, page_sensitive: true };

    table[0x85] = OpcodeInfo { mnemonic: "STA", addressing_mode: AddressingMode::ZeroPage,        cycles: 3, page_sensitive: false };
    table[0x95] = OpcodeInfo { mnemonic: "STA", addressing_mode: AddressingMode::ZeroPageX,       cycles: 4, page_sensitive: false };
    table[0x8D] = OpcodeInfo { mnemonic: "STA", addressing_mode: AddressingMode::Absolute,        cycles: 4, page_sensitive: false };
    table[0x9D] = OpcodeInfo { mnemonic: "STA", addressing_mode: AddressingMode::AbsoluteX,       cycles: 5, page_sensitive: false };
    table[0x99] = OpcodeInfo { mnemonic: "STA", addressing_mode: AddressingMode::AbsoluteY,       cycles: 5, page_sensitive: false };
    table[0x81] = OpcodeInfo { mnemonic: "STA", addressing_mode: AddressingMode::IndexedIndirect, cycles: 6, page_sensitive: false };
    table[0x91] = OpcodeInfo { mnemonic: "STA", addressing_mode: AddressingMode::IndirectIndexed, cycles: 6, page_sensitive: false };

    table[0x86] = OpcodeInfo { mnemonic: "STX", addressing_mode: AddressingMode::ZeroPage,        cycles: 3, page_sensitive: false };
    table[0x96] = OpcodeInfo { mnemonic: "STX", addressing_mode: AddressingMode::ZeroPageY,       cycles: 4, page_sensitive: false };
    table[0x8E] = OpcodeInfo { mnemonic: "STX", addressing_mode: AddressingMode::Absolute,        cycles: 4, page_sensitive: false };

    table[0x84] = OpcodeInfo { mnemonic: "STY", addressing_mode: AddressingMode::ZeroPage,        cycles: 3, page_sensitive: false };
    table[0x94] = OpcodeInfo { mnemonic: "STY", addressing_mode: AddressingMode::ZeroPageX,       cycles: 4, page_sensitive: false };
    table[0x8C] = OpcodeInfo { mnemonic: "STY", addressing_mode: AddressingMode::Absolute,        cycles: 4, page_sensitive: false };

    // ----- Register transfers -----
    table[0xAA] = OpcodeInfo { mnemonic: "TAX", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };
    table[0x8A] = OpcodeInfo { mnemonic: "TXA", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };
    table[0xA8] = OpcodeInfo { mnemonic: "TAY", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };
    table[0x98] = OpcodeInfo { mnemonic: "TYA", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };
    table[0xBA] = OpcodeInfo { mnemonic: "TSX", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };
    table[0x9A] = OpcodeInfo { mnemonic: "TXS", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };

    // ----- Stack -----
    table[0x48] = OpcodeInfo { mnemonic: "PHA", addressing_mode: AddressingMode::Implied,         cycles: 3, page_sensitive: false };
    table[0x68] = OpcodeInfo { mnemonic: "PLA", addressing_mode: AddressingMode::Implied,         cycles: 4, page_sensitive: false };
    table[0x08] = OpcodeInfo { mnemonic: "PHP", addressing_mode: AddressingMode::Implied,         cycles: 3, page_sensitive: false };
    table[0x28] = OpcodeInfo { mnemonic: "PLP", addressing_mode: AddressingMode::Implied,         cycles: 4, page_sensitive: false };

    // ----- Arithmetic / Compare -----
    table[0x69] = OpcodeInfo { mnemonic: "ADC", addressing_mode: AddressingMode::Immediate,       cycles: 2, page_sensitive: false };
    table[0x65] = OpcodeInfo { mnemonic: "ADC", addressing_mode: AddressingMode::ZeroPage,        cycles: 3, page_sensitive: false };
    table[0x75] = OpcodeInfo { mnemonic: "ADC", addressing_mode: AddressingMode::ZeroPageX,       cycles: 4, page_sensitive: false };
    table[0x6D] = OpcodeInfo { mnemonic: "ADC", addressing_mode: AddressingMode::Absolute,        cycles: 4, page_sensitive: false };
    table[0x7D] = OpcodeInfo { mnemonic: "ADC", addressing_mode: AddressingMode::AbsoluteX,       cycles: 4, page_sensitive: true };
    table[0x79] = OpcodeInfo { mnemonic: "ADC", addressing_mode: AddressingMode::AbsoluteY,       cycles: 4, page_sensitive: true };
    table[0x61] = OpcodeInfo { mnemonic: "ADC", addressing_mode: AddressingMode::IndexedIndirect, cycles: 6, page_sensitive: false };
    table[0x71] = OpcodeInfo { mnemonic: "ADC", addressing_mode: AddressingMode::IndirectIndexed, cycles: 5, page_sensitive: true };

    table[0xE9] = OpcodeInfo { mnemonic: "SBC", addressing_mode: AddressingMode::Immediate,       cycles: 2, page_sensitive: false };
    table[0xE5] = OpcodeInfo { mnemonic: "SBC", addressing_mode: AddressingMode::ZeroPage,        cycles: 3, page_sensitive: false };
    table[0xF5] = OpcodeInfo { mnemonic: "SBC", addressing_mode: AddressingMode::ZeroPageX,       cycles: 4, page_sensitive: false };
    table[0xED] = OpcodeInfo { mnemonic: "SBC", addressing_mode: AddressingMode::Absolute,        cycles: 4, page_sensitive: false };
    table[0xFD] = OpcodeInfo { mnemonic: "SBC", addressing_mode: AddressingMode::AbsoluteX,       cycles: 4, page_sensitive: true };
    table[0xF9] = OpcodeInfo { mnemonic: "SBC", addressing_mode: AddressingMode::AbsoluteY,       cycles: 4, page_sensitive: true };
    table[0xE1] = OpcodeInfo { mnemonic: "SBC", addressing_mode: AddressingMode::IndexedIndirect, cycles: 6, page_sensitive: false };
    table[0xF1] = OpcodeInfo { mnemonic: "SBC", addressing_mode: AddressingMode::IndirectIndexed, cycles: 5, page_sensitive: true };

    table[0xC9] = OpcodeInfo { mnemonic: "CMP", addressing_mode: AddressingMode::Immediate,       cycles: 2, page_sensitive: false };
    table[0xC5] = OpcodeInfo { mnemonic: "CMP", addressing_mode: AddressingMode::ZeroPage,        cycles: 3, page_sensitive: false };
    table[0xD5] = OpcodeInfo { mnemonic: "CMP", addressing_mode: AddressingMode::ZeroPageX,       cycles: 4, page_sensitive: false };
    table[0xCD] = OpcodeInfo { mnemonic: "CMP", addressing_mode: AddressingMode::Absolute,        cycles: 4, page_sensitive: false };
    table[0xDD] = OpcodeInfo { mnemonic: "CMP", addressing_mode: AddressingMode::AbsoluteX,       cycles: 4, page_sensitive: true };
    table[0xD9] = OpcodeInfo { mnemonic: "CMP", addressing_mode: AddressingMode::AbsoluteY,       cycles: 4, page_sensitive: true };
    table[0xC1] = OpcodeInfo { mnemonic: "CMP", addressing_mode: AddressingMode::IndexedIndirect, cycles: 6, page_sensitive: false };
    table[0xD1] = OpcodeInfo { mnemonic: "CMP", addressing_mode: AddressingMode::IndirectIndexed, cycles: 5, page_sensitive: true };

    table[0xE0] = OpcodeInfo { mnemonic: "CPX", addressing_mode: AddressingMode::Immediate,       cycles: 2, page_sensitive: false };
    table[0xE4] = OpcodeInfo { mnemonic: "CPX", addressing_mode: AddressingMode::ZeroPage,        cycles: 3, page_sensitive: false };
    table[0xEC] = OpcodeInfo { mnemonic: "CPX", addressing_mode: AddressingMode::Absolute,        cycles: 4, page_sensitive: false };

    table[0xC0] = OpcodeInfo { mnemonic: "CPY", addressing_mode: AddressingMode::Immediate,       cycles: 2, page_sensitive: false };
    table[0xC4] = OpcodeInfo { mnemonic: "CPY", addressing_mode: AddressingMode::ZeroPage,        cycles: 3, page_sensitive: false };
    table[0xCC] = OpcodeInfo { mnemonic: "CPY", addressing_mode: AddressingMode::Absolute,        cycles: 4, page_sensitive: false };

    // ----- Logical -----
    table[0x29] = OpcodeInfo { mnemonic: "AND", addressing_mode: AddressingMode::Immediate,       cycles: 2, page_sensitive: false };
    table[0x25] = OpcodeInfo { mnemonic: "AND", addressing_mode: AddressingMode::ZeroPage,        cycles: 3, page_sensitive: false };
    table[0x35] = OpcodeInfo { mnemonic: "AND", addressing_mode: AddressingMode::ZeroPageX,       cycles: 4, page_sensitive: false };
    table[0x2D] = OpcodeInfo { mnemonic: "AND", addressing_mode: AddressingMode::Absolute,        cycles: 4, page_sensitive: false };
    table[0x3D] = OpcodeInfo { mnemonic: "AND", addressing_mode: AddressingMode::AbsoluteX,       cycles: 4, page_sensitive: true };
    table[0x39] = OpcodeInfo { mnemonic: "AND", addressing_mode: AddressingMode::AbsoluteY,       cycles: 4, page_sensitive: true };
    table[0x21] = OpcodeInfo { mnemonic: "AND", addressing_mode: AddressingMode::IndexedIndirect, cycles: 6, page_sensitive: false };
    table[0x31] = OpcodeInfo { mnemonic: "AND", addressing_mode: AddressingMode::IndirectIndexed, cycles: 5, page_sensitive: true };

    table[0x09] = OpcodeInfo { mnemonic: "ORA", addressing_mode: AddressingMode::Immediate,       cycles: 2, page_sensitive: false };
    table[0x05] = OpcodeInfo { mnemonic: "ORA", addressing_mode: AddressingMode::ZeroPage,        cycles: 3, page_sensitive: false };
    table[0x15] = OpcodeInfo { mnemonic: "ORA", addressing_mode: AddressingMode::ZeroPageX,       cycles: 4, page_sensitive: false };
    table[0x0D] = OpcodeInfo { mnemonic: "ORA", addressing_mode: AddressingMode::Absolute,        cycles: 4, page_sensitive: false };
    table[0x1D] = OpcodeInfo { mnemonic: "ORA", addressing_mode: AddressingMode::AbsoluteX,       cycles: 4, page_sensitive: true };
    table[0x19] = OpcodeInfo { mnemonic: "ORA", addressing_mode: AddressingMode::AbsoluteY,       cycles: 4, page_sensitive: true };
    table[0x01] = OpcodeInfo { mnemonic: "ORA", addressing_mode: AddressingMode::IndexedIndirect, cycles: 6, page_sensitive: false };
    table[0x11] = OpcodeInfo { mnemonic: "ORA", addressing_mode: AddressingMode::IndirectIndexed, cycles: 5, page_sensitive: true };

    table[0x49] = OpcodeInfo { mnemonic: "EOR", addressing_mode: AddressingMode::Immediate,       cycles: 2, page_sensitive: false };
    table[0x45] = OpcodeInfo { mnemonic: "EOR", addressing_mode: AddressingMode::ZeroPage,        cycles: 3, page_sensitive: false };
    table[0x55] = OpcodeInfo { mnemonic: "EOR", addressing_mode: AddressingMode::ZeroPageX,       cycles: 4, page_sensitive: false };
    table[0x4D] = OpcodeInfo { mnemonic: "EOR", addressing_mode: AddressingMode::Absolute,        cycles: 4, page_sensitive: false };
    table[0x5D] = OpcodeInfo { mnemonic: "EOR", addressing_mode: AddressingMode::AbsoluteX,       cycles: 4, page_sensitive: true };
    table[0x59] = OpcodeInfo { mnemonic: "EOR", addressing_mode: AddressingMode::AbsoluteY,       cycles: 4, page_sensitive: true };
    table[0x41] = OpcodeInfo { mnemonic: "EOR", addressing_mode: AddressingMode::IndexedIndirect, cycles: 6, page_sensitive: false };
    table[0x51] = OpcodeInfo { mnemonic: "EOR", addressing_mode: AddressingMode::IndirectIndexed, cycles: 5, page_sensitive: true };

    table[0x24] = OpcodeInfo { mnemonic: "BIT", addressing_mode: AddressingMode::ZeroPage,        cycles: 3, page_sensitive: false };
    table[0x2C] = OpcodeInfo { mnemonic: "BIT", addressing_mode: AddressingMode::Absolute,        cycles: 4, page_sensitive: false };

    // ----- Shifts and rotates -----
    table[0x0A] = OpcodeInfo { mnemonic: "ASL", addressing_mode: AddressingMode::Accumulator,     cycles: 2, page_sensitive: false };
    table[0x06] = OpcodeInfo { mnemonic: "ASL", addressing_mode: AddressingMode::ZeroPage,        cycles: 5, page_sensitive: false };
    table[0x16] = OpcodeInfo { mnemonic: "ASL", addressing_mode: AddressingMode::ZeroPageX,       cycles: 6, page_sensitive: false };
    table[0x0E] = OpcodeInfo { mnemonic: "ASL", addressing_mode: AddressingMode::Absolute,        cycles: 6, page_sensitive: false };
    table[0x1E] = OpcodeInfo { mnemonic: "ASL", addressing_mode: AddressingMode::AbsoluteX,       cycles: 7, page_sensitive: false };

    table[0x4A] = OpcodeInfo { mnemonic: "LSR", addressing_mode: AddressingMode::Accumulator,     cycles: 2, page_sensitive: false };
    table[0x46] = OpcodeInfo { mnemonic: "LSR", addressing_mode: AddressingMode::ZeroPage,        cycles: 5, page_sensitive: false };
    table[0x56] = OpcodeInfo { mnemonic: "LSR", addressing_mode: AddressingMode::ZeroPageX,       cycles: 6, page_sensitive: false };
    table[0x4E] = OpcodeInfo { mnemonic: "LSR", addressing_mode: AddressingMode::Absolute,        cycles: 6, page_sensitive: false };
    table[0x5E] = OpcodeInfo { mnemonic: "LSR", addressing_mode: AddressingMode::AbsoluteX,       cycles: 7, page_sensitive: false };

    table[0x2A] = OpcodeInfo { mnemonic: "ROL", addressing_mode: AddressingMode::Accumulator,     cycles: 2, page_sensitive: false };
    table[0x26] = OpcodeInfo { mnemonic: "ROL", addressing_mode: AddressingMode::ZeroPage,        cycles: 5, page_sensitive: false };
    table[0x36] = OpcodeInfo { mnemonic: "ROL", addressing_mode: AddressingMode::ZeroPageX,       cycles: 6, page_sensitive: false };
    table[0x2E] = OpcodeInfo { mnemonic: "ROL", addressing_mode: AddressingMode::Absolute,        cycles: 6, page_sensitive: false };
    table[0x3E] = OpcodeInfo { mnemonic: "ROL", addressing_mode: AddressingMode::AbsoluteX,       cycles: 7, page_sensitive: false };

    table[0x6A] = OpcodeInfo { mnemonic: "ROR", addressing_mode: AddressingMode::Accumulator,     cycles: 2, page_sensitive: false };
    table[0x66] = OpcodeInfo { mnemonic: "ROR", addressing_mode: AddressingMode::ZeroPage,        cycles: 5, page_sensitive: false };
    table[0x76] = OpcodeInfo { mnemonic: "ROR", addressing_mode: AddressingMode::ZeroPageX,       cycles: 6, page_sensitive: false };
    table[0x6E] = OpcodeInfo { mnemonic: "ROR", addressing_mode: AddressingMode::Absolute,        cycles: 6, page_sensitive: false };
    table[0x7E] = OpcodeInfo { mnemonic: "ROR", addressing_mode: AddressingMode::AbsoluteX,       cycles: 7, page_sensitive: false };

    // ----- Increments and decrements -----
    table[0xE6] = OpcodeInfo { mnemonic: "INC", addressing_mode: AddressingMode::ZeroPage,        cycles: 5, page_sensitive: false };
    table[0xF6] = OpcodeInfo { mnemonic: "INC", addressing_mode: AddressingMode::ZeroPageX,       cycles: 6, page_sensitive: false };
    table[0xEE] = OpcodeInfo { mnemonic: "INC", addressing_mode: AddressingMode::Absolute,        cycles: 6, page_sensitive: false };
    table[0xFE] = OpcodeInfo { mnemonic: "INC", addressing_mode: AddressingMode::AbsoluteX,       cycles: 7, page_sensitive: false };

    table[0xC6] = OpcodeInfo { mnemonic: "DEC", addressing_mode: AddressingMode::ZeroPage,        cycles: 5, page_sensitive: false };
    table[0xD6] = OpcodeInfo { mnemonic: "DEC", addressing_mode: AddressingMode::ZeroPageX,       cycles: 6, page_sensitive: false };
    table[0xCE] = OpcodeInfo { mnemonic: "DEC", addressing_mode: AddressingMode::Absolute,        cycles: 6, page_sensitive: false };
    table[0xDE] = OpcodeInfo { mnemonic: "DEC", addressing_mode: AddressingMode::AbsoluteX,       cycles: 7, page_sensitive: false };

    table[0xE8] = OpcodeInfo { mnemonic: "INX", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };
    table[0xCA] = OpcodeInfo { mnemonic: "DEX", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };
    table[0xC8] = OpcodeInfo { mnemonic: "INY", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };
    table[0x88] = OpcodeInfo { mnemonic: "DEY", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };

    // ----- Flag operations -----
    table[0x18] = OpcodeInfo { mnemonic: "CLC", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };
    table[0x38] = OpcodeInfo { mnemonic: "SEC", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };
    table[0xD8] = OpcodeInfo { mnemonic: "CLD", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };
    table[0xF8] = OpcodeInfo { mnemonic: "SED", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };
    table[0x58] = OpcodeInfo { mnemonic: "CLI", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };
    table[0x78] = OpcodeInfo { mnemonic: "SEI", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };
    table[0xB8] = OpcodeInfo { mnemonic: "CLV", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };

    // ----- Branches -----
    table[0x90] = OpcodeInfo { mnemonic: "BCC", addressing_mode: AddressingMode::Relative,        cycles: 2, page_sensitive: false };
    table[0xB0] = OpcodeInfo { mnemonic: "BCS", addressing_mode: AddressingMode::Relative,        cycles: 2, page_sensitive: false };
    table[0xF0] = OpcodeInfo { mnemonic: "BEQ", addressing_mode: AddressingMode::Relative,        cycles: 2, page_sensitive: false };
    table[0xD0] = OpcodeInfo { mnemonic: "BNE", addressing_mode: AddressingMode::Relative,        cycles: 2, page_sensitive: false };
    table[0x30] = OpcodeInfo { mnemonic: "BMI", addressing_mode: AddressingMode::Relative,        cycles: 2, page_sensitive: false };
    table[0x10] = OpcodeInfo { mnemonic: "BPL", addressing_mode: AddressingMode::Relative,        cycles: 2, page_sensitive: false };
    table[0x50] = OpcodeInfo { mnemonic: "BVC", addressing_mode: AddressingMode::Relative,        cycles: 2, page_sensitive: false };
    table[0x70] = OpcodeInfo { mnemonic: "BVS", addressing_mode: AddressingMode::Relative,        cycles: 2, page_sensitive: false };

    // ----- Jumps and calls -----
    table[0x4C] = OpcodeInfo { mnemonic: "JMP", addressing_mode: AddressingMode::Absolute,        cycles: 3, page_sensitive: false };
    table[0x6C] = OpcodeInfo { mnemonic: "JMP", addressing_mode: AddressingMode::Indirect,        cycles: 5, page_sensitive: false };
    table[0x20] = OpcodeInfo { mnemonic: "JSR", addressing_mode: AddressingMode::Absolute,        cycles: 6, page_sensitive: false };
    table[0x60] = OpcodeInfo { mnemonic: "RTS", addressing_mode: AddressingMode::Implied,         cycles: 6, page_sensitive: false };

    // ----- System -----
    table[0x00] = OpcodeInfo { mnemonic: "BRK", addressing_mode: AddressingMode::Implied,         cycles: 7, page_sensitive: false };
    table[0xEA] = OpcodeInfo { mnemonic: "NOP", addressing_mode: AddressingMode::Implied,         cycles: 2, page_sensitive: false };
    table[0x40] = OpcodeInfo { mnemonic: "RTI", addressing_mode: AddressingMode::Implied,         cycles: 6, page_sensitive: false };

    table
};

/// The total length in bytes of the instruction whose opcode is `opcode`,
/// from its `OPCODES` row (`"???"` rows count as one byte).
#[must_use]
pub fn instruction_len_at(opcode: u8) -> u8 {
    OPCODES[usize::from(opcode)]
        .addressing_mode
        .instruction_len()
}

/// Route `opcode` to the handler for its instruction, passing the
/// addressing mode its `OPCODES` row supplies.
#[rustfmt::skip]
pub fn dispatch<B: Bus>(opcode: u8, cpu: &mut Chip6502, mode: AddressingMode, bus: &mut B) {
    match opcode {
        // ----- Load / Store -----
        0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => cpu.lda(mode, bus),
        0xA2 | 0xA6 | 0xB6 | 0xAE | 0xBE                     => cpu.ldx(mode, bus),
        0xA0 | 0xA4 | 0xB4 | 0xAC | 0xBC                     => cpu.ldy(mode, bus),
        0x85 | 0x95 | 0x8D | 0x9D | 0x99 | 0x81 | 0x91       => cpu.sta(mode, bus),
        0x86 | 0x96 | 0x8E                                   => cpu.stx(mode, bus),
        0x84 | 0x94 | 0x8C                                   => cpu.sty(mode, bus),

        // ----- Register transfers -----
        0xAA => cpu.tax(mode, bus),
        0x8A => cpu.txa(mode, bus),
        0xA8 => cpu.tay(mode, bus),
        0x98 => cpu.tya(mode, bus),
        0xBA => cpu.tsx(mode, bus),
        0x9A => cpu.txs(mode, bus),

        // ----- Stack -----
        0x48 => cpu.pha(mode, bus),
        0x68 => cpu.pla(mode, bus),
        0x08 => cpu.php(mode, bus),
        0x28 => cpu.plp(mode, bus),

        // ----- Arithmetic / Compare -----
        0x69 | 0x65 | 0x75 | 0x6D | 0x7D | 0x79 | 0x61 | 0x71 => cpu.adc(mode, bus),
        0xE9 | 0xE5 | 0xF5 | 0xED | 0xFD | 0xF9 | 0xE1 | 0xF1 => cpu.sbc(mode, bus),
        0xC9 | 0xC5 | 0xD5 | 0xCD | 0xDD | 0xD9 | 0xC1 | 0xD1 => cpu.cmp(mode, bus),
        0xE0 | 0xE4 | 0xEC                                   => cpu.cpx(mode, bus),
        0xC0 | 0xC4 | 0xCC                                   => cpu.cpy(mode, bus),

        // ----- Logical -----
        0x29 | 0x25 | 0x35 | 0x2D | 0x3D | 0x39 | 0x21 | 0x31 => cpu.and(mode, bus),
        0x09 | 0x05 | 0x15 | 0x0D | 0x1D | 0x19 | 0x01 | 0x11 => cpu.ora(mode, bus),
        0x49 | 0x45 | 0x55 | 0x4D | 0x5D | 0x59 | 0x41 | 0x51 => cpu.eor(mode, bus),
        0x24 | 0x2C                                          => cpu.bit(mode, bus),

        // ----- Shifts and rotates -----
        0x0A                       => cpu.asl_accumulator(mode, bus),
        0x06 | 0x16 | 0x0E | 0x1E  => cpu.asl(mode, bus),
        0x4A                       => cpu.lsr_accumulator(mode, bus),
        0x46 | 0x56 | 0x4E | 0x5E  => cpu.lsr(mode, bus),
        0x2A                       => cpu.rol_accumulator(mode, bus),
        0x26 | 0x36 | 0x2E | 0x3E  => cpu.rol(mode, bus),
        0x6A                       => cpu.ror_accumulator(mode, bus),
        0x66 | 0x76 | 0x6E | 0x7E  => cpu.ror(mode, bus),

        // ----- Increments and decrements -----
        0xE6 | 0xF6 | 0xEE | 0xFE  => cpu.inc(mode, bus),
        0xC6 | 0xD6 | 0xCE | 0xDE  => cpu.dec(mode, bus),
        0xE8 => cpu.inx(mode, bus),
        0xCA => cpu.dex(mode, bus),
        0xC8 => cpu.iny(mode, bus),
        0x88 => cpu.dey(mode, bus),

        // ----- Flag operations -----
        0x18 => cpu.clc(mode, bus),
        0x38 => cpu.sec(mode, bus),
        0xD8 => cpu.cld(mode, bus),
        0xF8 => cpu.sed(mode, bus),
        0x58 => cpu.cli(mode, bus),
        0x78 => cpu.sei(mode, bus),
        0xB8 => cpu.clv(mode, bus),

        // ----- Branches -----
        0x90 => cpu.bcc_relative(mode, bus),
        0xB0 => cpu.bcs_relative(mode, bus),
        0xF0 => cpu.beq_relative(mode, bus),
        0xD0 => cpu.bne_relative(mode, bus),
        0x30 => cpu.bmi_relative(mode, bus),
        0x10 => cpu.bpl_relative(mode, bus),
        0x50 => cpu.bvc_relative(mode, bus),
        0x70 => cpu.bvs_relative(mode, bus),

        // ----- Jumps and calls -----
        0x4C => cpu.jmp_absolute(mode, bus),
        0x6C => cpu.jmp_indirect(mode, bus),
        0x20 => cpu.jsr_absolute(mode, bus),
        0x60 => cpu.rts(mode, bus),

        // ----- System -----
        0x00 => cpu.brk(mode, bus),
        0xEA => cpu.nop(mode, bus),
        0x40 => cpu.rti(mode, bus),

        _ => cpu.not_implemented(mode, bus),
    }
}
