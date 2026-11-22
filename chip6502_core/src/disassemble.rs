// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! The disassembler: opcode bytes back into readable 6502 assembly.
//!
//! `disassemble_at` formats the one instruction at an address, and
//! `disassemble_range` walks a range, stepping by each instruction's length.
//! Both read the same `OPCODES` rows that execution reads, so the trace and
//! the processor cannot disagree about what an opcode is. Branches print
//! their resolved target rather than the raw offset, because the target is
//! the question you ask when a loop misbehaves.
//!
//! Introduced in Build 3 — Interlude: The Disassembler; its private opcode
//! match was retired into the shared `OPCODES` table in Build 22.

use crate::addressing::AddressingMode;
use crate::opcodes::{instruction_len_at, OPCODES};
use crate::Bus;

/// Format the instruction at `pc` as a one-line disassembly string.
#[rustfmt::skip]
#[expect(
    clippy::uninlined_format_args,
    reason = "Build 3 prints the format strings with positional arguments"
)]
#[expect(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "the Relative arm reuses Build 1's two branch casts"
)]
pub fn disassemble_at<B: Bus>(pc: u16, bus: &mut B) -> String {
    let info = &OPCODES[usize::from(bus.memory_read(pc))];
    let (mnemonic, mode) = (info.mnemonic, info.addressing_mode);
    let operand = bus.memory_read(pc.wrapping_add(1));
    let word = u16::from_le_bytes([operand, bus.memory_read(pc.wrapping_add(2))]);
    match mode {
        AddressingMode::Implied => format!("{:04X}: {} ({:?})", pc, mnemonic, mode),
        AddressingMode::Accumulator => format!("{:04X}: {} A ({:?})", pc, mnemonic, mode),
        AddressingMode::Immediate => format!("{:04X}: {} #${:02X} ({:?})", pc, mnemonic, operand, mode),
        AddressingMode::ZeroPage => format!("{:04X}: {} ${:02X} ({:?})", pc, mnemonic, operand, mode),
        AddressingMode::ZeroPageX => format!("{:04X}: {} ${:02X},X ({:?})", pc, mnemonic, operand, mode),
        AddressingMode::ZeroPageY => format!("{:04X}: {} ${:02X},Y ({:?})", pc, mnemonic, operand, mode),
        AddressingMode::Absolute => format!("{:04X}: {} ${:04X} ({:?})", pc, mnemonic, word, mode),
        AddressingMode::AbsoluteX => format!("{:04X}: {} ${:04X},X ({:?})", pc, mnemonic, word, mode),
        AddressingMode::AbsoluteY => format!("{:04X}: {} ${:04X},Y ({:?})", pc, mnemonic, word, mode),
        AddressingMode::Indirect => format!("{:04X}: {} (${:04X}) ({:?})", pc, mnemonic, word, mode),
        AddressingMode::IndexedIndirect => format!("{:04X}: {} (${:02X},X) ({:?})", pc, mnemonic, operand, mode),
        AddressingMode::IndirectIndexed => format!("{:04X}: {} (${:02X}),Y ({:?})", pc, mnemonic, operand, mode),
        AddressingMode::Relative => {
            let offset = operand as i8;
            let target = pc.wrapping_add(2).wrapping_add(offset as u16);
            format!("{:04X}: {} ${:04X} ({:?})", pc, mnemonic, target, mode)
        }
    }
}

/// Print one disassembled line per instruction from `start` up to, but not
/// including, `end`, stepping by each instruction's length.
pub fn disassemble_range(start: u16, end: u16, bus: &mut impl Bus) {
    let mut pc = start;
    while pc < end {
        println!("{}", disassemble_at(pc, bus));
        // checked_add, not wrapping_add: stepping past $FFFF ends the
        // listing instead of wrapping to $0000 and starting again.
        match pc.checked_add(u16::from(instruction_len_at(bus.memory_read(pc)))) {
            Some(next) => pc = next,
            None => break,
        }
    }
}
