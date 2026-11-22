// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! The Chapter 4 scaffold program: print one character through the core.
//!
//! Assembles eight bytes by hand (`LDA #'H'`, `STA $F001`, `JMP` to self),
//! points the reset vector at them, and steps the CPU until the program
//! halts. The character reaches the terminal only through the final
//! `println!`: the core captures output and the application decides what
//! to do with it.
//!
//! Introduced in Chapter 4 — Rust and System Programming.

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use chip6502_core::{Chip6502, ConsoleBus};

#[rustfmt::skip]
fn main() {
    let mut bus = ConsoleBus::new();
    let mut cpu = Chip6502::new();

    let program = [
        0xA9, 0x48,        // LDA #$48      'H'
        0x8D, 0x01, 0xF0,  // STA $F001     write to the console port
        0x4C, 0x05, 0x02,  // JMP $0205     halt: jump to self
    ];
    bus.ram.memory[0x0200..0x0200 + program.len()].copy_from_slice(&program);

    bus.ram.memory[0xFFFC] = 0x00;    // reset vector -> $0200
    bus.ram.memory[0xFFFD] = 0x02;

    cpu.reset(&mut bus);
    loop {
        let pc_before = cpu.pc;
        cpu.step(&mut bus);
        if cpu.pc == pc_before {
            break;                 // JMP to itself: the program has halted
        }
    }

    println!("{}", String::from_utf8_lossy(&bus.output_log));
}
