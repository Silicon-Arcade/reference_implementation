// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! Functional tests: each build's whole program, run to its halt.
//!
//! Every test loads a build's machine code at `$0200` on a `ConsoleBus`,
//! runs it until PC stops moving (the `JMP`-to-self halt), and compares the
//! bytes written to `$F001` with the build's *Expected output*, byte for
//! byte. The comment above each test restates the build's instruction table
//! so a failure names its suspects, and the `fnl_` prefix lets
//! `cargo test fnl_` run this layer alone.
//!
//! Introduced in Build 1 — Hello, world; each program build through Build 25 adds its own test.

use crate::{Chip6502, ConsoleBus};

/// Copy `program` to `$0200`, point PC at it, and run to the `JMP`-self halt.
fn load_and_run_program(cpu: &mut Chip6502, bus: &mut ConsoleBus, program: &[u8]) {
    bus.ram.memory[0x0200..0x0200 + program.len()].copy_from_slice(program);
    cpu.pc = 0x0200;

    for _ in 0..100_000 {
        let pc_before = cpu.pc;
        cpu.step(bus);
        if cpu.pc == pc_before {
            return; // JMP-to-self: the program has halted
        }
    }
    panic!("still running after 100,000 instructions — runaway program?");
}

/// Build 1 — Hello, world.
// Exercises, in combination:
//   A2 LDX #imm · BD LDA abs,X · F0 BEQ rel · 8D STA abs
//   E8 INX      · D0 BNE rel   · 4C JMP abs
#[test]
#[rustfmt::skip]
fn fnl_hello_world() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    let program = [
        0xA2, 0x00,                                          // LDX #0
        0xBD, 0x10, 0x02,                                    // LDA MSG,X
        0xF0, 0x06,                                          // BEQ DONE
        0x8D, 0x01, 0xF0,                                    // STA $F001
        0xE8,                                                // INX
        0xD0, 0xF5,                                          // BNE LOOP
        0x4C, 0x0D, 0x02,                                    // JMP DONE
        0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x2C, 0x20,            // "Hello, "
        0x77, 0x6F, 0x72, 0x6C, 0x64, 0x21,                  // "world!"
        0x0A, 0x00,                                          // LF, NUL
    ];

    load_and_run_program(&mut cpu, &mut bus, &program);

    assert_eq!(bus.output_log, b"Hello, world!\n");
}

/// Build 2 — Count 0 to 9 with subroutines.
// Exercises, in combination:
//   8A TXA · 18 CLC · 69 ADC #imm · E0 CPX #imm · 20 JSR abs · 60 RTS
//   (plus Build 1's LDX, INX, BNE, JMP, STA)
#[test]
#[rustfmt::skip]
fn fnl_count_0_to_9() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    let count_0_to_9 = [
        0xA2, 0x00,             // LDX #$00
        0x8A,                   // TXA
        0x18,                   // CLC
        0x69, 0x30,             // ADC #$30
        0x20, 0x11, 0x02,       // JSR $0211
        0xE8,                   // INX
        0xE0, 0x0A,             // CPX #$0A
        0xD0, 0xF4,             // BNE $0202
        0x4C, 0x0E, 0x02,       // JMP $020E (halt)
        0x8D, 0x01, 0xF0,       // STA $F001    ← PRINT_CHAR @ $0211
        0x60,                   // RTS
    ];

    load_and_run_program(&mut cpu, &mut bus, &count_0_to_9);

    assert_eq!(bus.output_log, b"0123456789");
}

/// Build 4 — Print via zero-page indirect pointer.
// Exercises, in combination:
//   85 STA zp · A0 LDY #imm · C8 INY · B1 LDA (zp),Y
//   (plus LDA #imm, JSR, RTS, BEQ, BNE, STA abs, JMP)
#[test]
#[rustfmt::skip]
fn fnl_zeropage_print() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    let print_via_pointer = [
        // mainline
        0xA9, 0x26, 0x85, 0x80, 0xA9, 0x02, 0x85, 0x81,    // setup ptr to MSG1
        0x20, 0x19, 0x02,                                    // JSR PRINT_STRING
        0xA9, 0x2E, 0x85, 0x80, 0xA9, 0x02, 0x85, 0x81,    // setup ptr to MSG2
        0x20, 0x19, 0x02,                                    // JSR PRINT_STRING
        0x4C, 0x16, 0x02,                                    // JMP halt
        // PRINT_STRING subroutine
        0xA0, 0x00,                                          // LDY #0
        0xB1, 0x80, 0xF0, 0x06, 0x8D, 0x01, 0xF0,           // .LOOP: read, BEQ, STA
        0xC8, 0xD0, 0xF6,                                    // INY, BNE .LOOP
        0x60,                                                // RTS
        // data
        0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x2C, 0x20, 0x00,    // "Hello, "
        0x77, 0x6F, 0x72, 0x6C, 0x64, 0x21, 0x0A, 0x00,    // "world!\n"
    ];

    load_and_run_program(&mut cpu, &mut bus, &print_via_pointer);

    assert_eq!(bus.output_log, b"Hello, world!\n");
}

/// Build 5 — Hex byte output.
// Exercises, in combination:
//   48 PHA · 68 PLA · 4A LSR A · 29 AND #imm · C9 CMP #imm · 90 BCC rel
//   (plus LDA, CLC, ADC, JSR, RTS, STA, JMP)
#[test]
#[rustfmt::skip]
fn fnl_hex_output() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    let hex_byte = [
        0xA9, 0xAB,                                  // LDA #$AB
        0x20, 0x0D, 0x02,                            // JSR PRINT_HEX_BYTE
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,                // LDA #LF, STA $F001
        0x4C, 0x0A, 0x02,                            // JMP halt
        // PRINT_HEX_BYTE
        0x48, 0x4A, 0x4A, 0x4A, 0x4A, 0x20, 0x1C, 0x02,
        0x68, 0x29, 0x0F, 0x20, 0x1C, 0x02, 0x60,
        // PRINT_NIBBLE
        0xC9, 0x0A, 0x90, 0x06, 0x18, 0x69, 0x37, 0x4C, 0x29, 0x02,
        0x18, 0x69, 0x30, 0x8D, 0x01, 0xF0, 0x60,
    ];

    load_and_run_program(&mut cpu, &mut bus, &hex_byte);

    assert_eq!(bus.output_log, b"AB\n");
}

/// Build 6 — Memory dump.
// Exercises, in combination:
//   A5 LDA zp · C0 CPY #imm · E6 INC zp
//   (plus Builds 4 and 5: LDA (zp),Y, PRINT_HEX_BYTE and its opcodes)
#[test]
#[rustfmt::skip]
fn fnl_memory_dump() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    let memory_dump = [
        // Setup
        0xA9, 0x00, 0x85, 0x80, 0xA9, 0x02, 0x85, 0x81, 0xA2, 0x00,
        // LINE_LOOP: print address
        0xA5, 0x81, 0x20, 0x47, 0x02,                    // LDA $81; JSR PRINT_HEX_BYTE
        0xA5, 0x80, 0x20, 0x47, 0x02,                    // LDA $80; JSR PRINT_HEX_BYTE
        0xA9, 0x3A, 0x8D, 0x01, 0xF0,                    // print ':'
        0xA9, 0x20, 0x8D, 0x01, 0xF0,                    // print ' '
        // BYTE_LOOP: print 8 bytes
        0xA0, 0x00,                                       // LDY #0
        0xB1, 0x80, 0x20, 0x47, 0x02,                    // LDA ($80),Y; JSR PRINT_HEX_BYTE
        0xA9, 0x20, 0x8D, 0x01, 0xF0,                    // print ' '
        0xC8, 0xC0, 0x08, 0xD0, 0xF1,                    // INY; CPY #8; BNE BYTE_LOOP
        // newline
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,
        // pointer += 8 (with carry)
        0x18, 0xA5, 0x80, 0x69, 0x08, 0x85, 0x80, 0x90, 0x02, 0xE6, 0x81,
        // line counter
        0xE8, 0xE0, 0x04, 0xD0, 0xC6,
        // halt
        0x4C, 0x44, 0x02,
        // PRINT_HEX_BYTE @ $0247
        0x48, 0x4A, 0x4A, 0x4A, 0x4A, 0x20, 0x56, 0x02,
        0x68, 0x29, 0x0F, 0x20, 0x56, 0x02, 0x60,
        // PRINT_NIBBLE @ $0256
        0xC9, 0x0A, 0x90, 0x06, 0x18, 0x69, 0x37, 0x4C, 0x63, 0x02,
        0x18, 0x69, 0x30, 0x8D, 0x01, 0xF0, 0x60,
    ];

    load_and_run_program(&mut cpu, &mut bus, &memory_dump);

    let expected: &[u8] =
        b"0200: A9 00 85 80 A9 02 85 81 \n\
          0208: A2 00 A5 81 20 47 02 A5 \n\
          0210: 80 20 47 02 A9 3A 8D 01 \n\
          0218: F0 A9 20 8D 01 F0 A0 00 \n";
    assert_eq!(bus.output_log, expected);
}

/// Build 8 — Print 99 down to 0 in decimal.
// Exercises, in combination:
//   AA TAX · CA DEX · 10 BPL rel · E9 SBC #imm
//   (plus TXA/PHA/PLA, CMP #imm, BCC, BEQ, CLC/ADC, JSR/RTS)
#[test]
#[rustfmt::skip]
fn fnl_count_99_to_0() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    let countdown_decimal = [
        0xA2, 0x63,                                 // LDX #99
        // LOOP at $0202
        0x8A, 0x48, 0x20, 0x19, 0x02,               // TXA; PHA; JSR PRINT_DEC
        0xA9, 0x20, 0x8D, 0x01, 0xF0,               // print ' '
        0x68, 0xAA, 0xCA, 0x10, 0xF1,               // PLA; TAX; DEX; BPL LOOP
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,               // print '\n'
        0x4C, 0x16, 0x02,                           // JMP halt
        // PRINT_DEC at $0219
        0xA2, 0x00,                                 // LDX #0
        // TENS_LOOP at $021B
        0xC9, 0x0A, 0x90, 0x06, 0xE9, 0x0A, 0xE8,   // CMP/BCC/SBC/INX
        0x4C, 0x1B, 0x02,                           // JMP TENS_LOOP
        // TENS_DONE at $0225
        0x48, 0x8A, 0xF0, 0x06,                     // PHA; TXA; BEQ SKIP_TENS
        0x18, 0x69, 0x30, 0x8D, 0x01, 0xF0,         // CLC; ADC #'0'; STA $F001
        // SKIP_TENS at $022F
        0x68, 0x18, 0x69, 0x30, 0x8D, 0x01, 0xF0,   // PLA; CLC; ADC #'0'; STA $F001
        0x60,                                       // RTS
    ];

    load_and_run_program(&mut cpu, &mut bus, &countdown_decimal);

    assert_eq!(
        bus.output_log,
        b"99 98 97 96 95 94 93 92 91 90 89 88 87 86 85 84 83 82 81 80 79 78 77 76 75 74 73 72 71 70 69 68 67 66 65 64 63 62 61 60 59 58 57 56 55 54 53 52 51 50 49 48 47 46 45 44 43 42 41 40 39 38 37 36 35 34 33 32 31 30 29 28 27 26 25 24 23 22 21 20 19 18 17 16 15 14 13 12 11 10 9 8 7 6 5 4 3 2 1 0 \n"
    );
}

/// Build 9 — Pseudo-random number generator (LFSR).
// Exercises, in combination:
//   0A ASL A · 49 EOR #imm
//   (plus STA/LDA zp, DEX/BNE, BCC, and Build 5's PRINT_HEX_BYTE)
#[test]
#[rustfmt::skip]
fn fnl_lfsr() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    let lfsr_random = [
        // Setup
        0xA9, 0x01, 0x85, 0x80, 0xA2, 0x10,           // LDA #1; STA $80; LDX #16
        // LOOP @ $0206
        0x20, 0x1C, 0x02,                              // JSR RND
        0x20, 0x26, 0x02,                              // JSR PRINT_HEX_BYTE
        0xA9, 0x20, 0x8D, 0x01, 0xF0,                  // print ' '
        0xCA, 0xD0, 0xF2,                              // DEX; BNE LOOP
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,                  // print '\n'
        0x4C, 0x19, 0x02,                              // JMP halt
        // RND @ $021C
        0xA5, 0x80, 0x0A, 0x90, 0x02, 0x49, 0x1D, 0x85, 0x80, 0x60,
        // PRINT_HEX_BYTE @ $0226
        0x48, 0x4A, 0x4A, 0x4A, 0x4A, 0x20, 0x35, 0x02,
        0x68, 0x29, 0x0F, 0x20, 0x35, 0x02, 0x60,
        // PRINT_NIBBLE @ $0235
        0xC9, 0x0A, 0x90, 0x06, 0x18, 0x69, 0x37, 0x4C, 0x42, 0x02,
        0x18, 0x69, 0x30, 0x8D, 0x01, 0xF0, 0x60,
    ];

    load_and_run_program(&mut cpu, &mut bus, &lfsr_random);

    assert_eq!(bus.output_log, b"02 04 08 10 20 40 80 1D 3A 74 E8 CD 87 13 26 4C \n");
}

/// Build 10 — Multiplication via shift-and-add.
// Exercises, in combination:
//   06 ASL zp · 65 ADC zp · 88 DEY
//   (plus ASL A, BCC, CLC, and Build 8's PRINT_DEC)
#[test]
#[rustfmt::skip]
fn fnl_multiplication() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    let multiplication = [
        // Setup
        0xA2, 0x01,                                          // LDX #1
        // LOOP @ $0202
        0x8A, 0x48,                                          // TXA; PHA
        0x85, 0x80, 0x85, 0x81,                              // STA $80; STA $81
        0x20, 0x22, 0x02,                                    // JSR MULT
        0x20, 0x32, 0x02,                                    // JSR PRINT_DEC
        0xA9, 0x20, 0x8D, 0x01, 0xF0,                        // print ' '
        0x68, 0xAA,                                          // PLA; TAX
        0xE8, 0xE0, 0x0A, 0xD0, 0xE8,                        // INX; CPX #10; BNE LOOP
        // newline + halt
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,
        0x4C, 0x1F, 0x02,                                    // JMP halt
        // MULT @ $0222
        0xA9, 0x00, 0xA0, 0x08,                              // LDA #0; LDY #8
        0x0A, 0x06, 0x81,                                    // ASL A; ASL $81
        0x90, 0x03, 0x18, 0x65, 0x80,                        // BCC NA; CLC; ADC $80
        0x88, 0xD0, 0xF5, 0x60,                              // DEY; BNE ML; RTS
        // PRINT_DEC @ $0232
        0xA2, 0x00,                                          // LDX #0
        0xC9, 0x0A, 0x90, 0x06, 0xE9, 0x0A, 0xE8,            // CMP/BCC/SBC/INX
        0x4C, 0x34, 0x02,                                    // JMP TENS_LOOP
        0x48, 0x8A, 0xF0, 0x06,                              // PHA; TXA; BEQ SKIP_TENS
        0x18, 0x69, 0x30, 0x8D, 0x01, 0xF0,                  // CLC; ADC #'0'; STA $F001
        0x68, 0x18, 0x69, 0x30, 0x8D, 0x01, 0xF0,            // PLA; CLC; ADC #'0'; STA $F001
        0x60,                                                // RTS
    ];

    load_and_run_program(&mut cpu, &mut bus, &multiplication);

    assert_eq!(bus.output_log, b"1 4 9 16 25 36 49 64 81 \n");
}

/// Build 11's program, 107 bytes; the first instruction picks the handler.
#[rustfmt::skip]
const JUMP_TABLE_DISPATCH: [u8; 107] = [
    // Dispatcher @ $0200
    0xA9, 0x01,                          // LDA #1
    0x0A, 0xAA,                          // ASL A; TAX
    0xBD, 0x14, 0x02, 0x85, 0x80,        // LDA HANDLERS,X; STA $80
    0xBD, 0x15, 0x02, 0x85, 0x81,        // LDA HANDLERS+1,X; STA $81
    0x6C, 0x80, 0x00,                    // JMP ($0080)  ← JMP INDIRECT
    // DONE @ $0211
    0x4C, 0x11, 0x02,                    // JMP $0211 (halt)
    // HANDLERS @ $0214 (3 entries × 2 bytes)
    0x1A, 0x02,                          // -> HANDLE_HELLO
    0x28, 0x02,                          // -> HANDLE_HEX
    0x30, 0x02,                          // -> HANDLE_BAD
    // HANDLE_HELLO @ $021A
    0xA9, 0x65, 0x85, 0x80,              // LDA #<MSG; STA $80
    0xA9, 0x02, 0x85, 0x81,              // LDA #>MSG; STA $81
    0x20, 0x38, 0x02,                    // JSR PRINT_STRING
    0x4C, 0x11, 0x02,                    // JMP DONE
    // HANDLE_HEX @ $0228
    0xA9, 0xAB,                          // LDA #$AB
    0x20, 0x45, 0x02,                    // JSR PRINT_HEX_BYTE
    0x4C, 0x11, 0x02,                    // JMP DONE
    // HANDLE_BAD @ $0230
    0xA9, 0x3F,                          // LDA #'?'
    0x8D, 0x01, 0xF0,                    // STA $F001
    0x4C, 0x11, 0x02,                    // JMP DONE
    // PRINT_STRING @ $0238
    0xA0, 0x00,                          // LDY #0
    0xB1, 0x80, 0xF0, 0x06,              // LDA ($80),Y; BEQ end
    0x8D, 0x01, 0xF0,                    // STA $F001
    0xC8, 0xD0, 0xF6,                    // INY; BNE loop
    0x60,                                 // RTS
    // PRINT_HEX_BYTE @ $0245
    0x48, 0x4A, 0x4A, 0x4A, 0x4A,        // PHA; LSR A x4
    0x20, 0x54, 0x02,                    // JSR PRINT_NIBBLE
    0x68, 0x29, 0x0F,                    // PLA; AND #$0F
    0x20, 0x54, 0x02,                    // JSR PRINT_NIBBLE
    0x60,                                 // RTS
    // PRINT_NIBBLE @ $0254
    0xC9, 0x0A, 0x90, 0x06,              // CMP #$0A; BCC IS_DIGIT
    0x18, 0x69, 0x37, 0x4C, 0x61, 0x02,  // CLC; ADC #$37; JMP OUT
    0x18, 0x69, 0x30,                    // IS_DIGIT: CLC; ADC #$30
    0x8D, 0x01, 0xF0,                    // OUT: STA $F001
    0x60,                                 // RTS
    // MSG @ $0265
    0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x00,  // "Hello\0"
];

/// Build 11 — Jump-table dispatch, command 0 (`LDA #0`).
// Exercises, in combination:
//   6C JMP (ind) · BD LDA abs,X · 0A ASL A · AA TAX · Build 4's PRINT_STRING
#[test]
fn fnl_jump_table_hello() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();
    let mut program = JUMP_TABLE_DISPATCH;
    program[1] = 0x00; // LDA #0 dispatches to HANDLE_HELLO

    load_and_run_program(&mut cpu, &mut bus, &program);

    assert_eq!(bus.output_log, b"Hello");
}

/// Build 11 — Jump-table dispatch, command 1 (`LDA #1`, as printed).
// Exercises, in combination:
//   6C JMP (ind) · BD LDA abs,X · 0A ASL A · AA TAX · Build 5's PRINT_HEX_BYTE
#[test]
fn fnl_jump_table_hex() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    load_and_run_program(&mut cpu, &mut bus, &JUMP_TABLE_DISPATCH);

    assert_eq!(bus.output_log, b"AB");
}

/// Build 12 — 16-bit decimal output (0–65535).
// Exercises, in combination:
//   38 SEC · the two-byte SBC chain (E9 SBC #imm, BCC) · STA/LDA zp
//   (plus TXA, BNE/BEQ, CLC/ADC, JSR/RTS)
#[test]
#[rustfmt::skip]
fn fnl_16_bit_decimal_output() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    let decimal_16bit = [
        // Setup @ $0200
        0xA9, 0x39, 0x85, 0x80, 0xA9, 0x30, 0x85, 0x81,        // value = $3039
        0xA9, 0x00, 0x85, 0x90,                                  // clear leading-zero flag
        // 10000s @ $020C
        0xA2, 0x00,                                              // LDX #0
        0x38, 0xA5, 0x80, 0xE9, 0x10, 0x85, 0xA0,                // SEC; LDA $80; SBC #$10; STA $A0
        0xA5, 0x81, 0xE9, 0x27, 0x90, 0x0A,                      // LDA $81; SBC #$27; BCC done
        0x85, 0x81, 0xA5, 0xA0, 0x85, 0x80,                      // commit
        0xE8, 0x4C, 0x0E, 0x02,                                  // INX; JMP TEN_K_LOOP
        0x20, 0x8C, 0x02,                                        // JSR PRINT_DIGIT
        // 1000s @ $0228
        0xA2, 0x00,
        0x38, 0xA5, 0x80, 0xE9, 0xE8, 0x85, 0xA0,
        0xA5, 0x81, 0xE9, 0x03, 0x90, 0x0A,
        0x85, 0x81, 0xA5, 0xA0, 0x85, 0x80,
        0xE8, 0x4C, 0x2A, 0x02,
        0x20, 0x8C, 0x02,
        // 100s @ $0244
        0xA2, 0x00,
        0x38, 0xA5, 0x80, 0xE9, 0x64, 0x85, 0xA0,
        0xA5, 0x81, 0xE9, 0x00, 0x90, 0x0A,
        0x85, 0x81, 0xA5, 0xA0, 0x85, 0x80,
        0xE8, 0x4C, 0x46, 0x02,
        0x20, 0x8C, 0x02,
        // 10s @ $0260
        0xA2, 0x00,
        0x38, 0xA5, 0x80, 0xE9, 0x0A, 0x85, 0xA0,
        0xA5, 0x81, 0xE9, 0x00, 0x90, 0x0A,
        0x85, 0x81, 0xA5, 0xA0, 0x85, 0x80,
        0xE8, 0x4C, 0x62, 0x02,
        0x20, 0x8C, 0x02,
        // Units @ $027C
        0xA5, 0x80, 0x18, 0x69, 0x30, 0x8D, 0x01, 0xF0,
        // Newline + halt @ $0284
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,
        0x4C, 0x89, 0x02,
        // PRINT_DIGIT @ $028C
        0x8A, 0xD0, 0x04, 0xA5, 0x90, 0xF0, 0x0B,
        0xA9, 0x01, 0x85, 0x90, 0x8A, 0x18, 0x69, 0x30, 0x8D, 0x01, 0xF0,
        0x60,
    ];

    load_and_run_program(&mut cpu, &mut bus, &decimal_16bit);
    assert_eq!(bus.output_log, b"12345\n");

    // The other rows of the Expected output table: change the two setup bytes.
    for (value, expected) in [
        (0x0000_u16, &b"0\n"[..]), (0x0007, b"7\n"), (0x007B, b"123\n"),
        (0xFFFF, b"65535\n"), (0x00FF, b"255\n"), (0x2710, b"10000\n"),
    ] {
        let [lo, hi] = value.to_le_bytes();
        let mut program = decimal_16bit;
        program[1] = lo;
        program[5] = hi;
        let mut cpu = Chip6502::new();
        let mut bus = ConsoleBus::new();
        load_and_run_program(&mut cpu, &mut bus, &program);
        assert_eq!(bus.output_log, expected, "value ${value:04X}");
    }
}

/// Build 13 — Multi-byte arithmetic (16-bit add and subtract).
// Exercises, in combination:
//   E5 SBC zp · the CLC/ADC zp and SEC/SBC zp chains · PRINT_HEX_BYTE
#[test]
#[rustfmt::skip]
fn fnl_multi_byte_arithmetic() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    let multi_byte_arithmetic = [
        // Setup @ $0200
        0xA9, 0x34, 0x85, 0x80, 0xA9, 0x12, 0x85, 0x81,         // A = $1234
        0xA9, 0x89, 0x85, 0x82, 0xA9, 0x00, 0x85, 0x83,         // B = $0089

        // ADD compute @ $0210
        0x18, 0xA5, 0x80, 0x65, 0x82, 0x85, 0x84,                // CLC; LDA $80; ADC $82; STA $84
        0xA5, 0x81, 0x65, 0x83, 0x85, 0x85,                      // LDA $81; ADC $83; STA $85

        // ADD print line @ $021D
        0xA5, 0x81, 0x20, 0xAF, 0x02,                            // LDA $81; JSR PRINT_HEX_BYTE
        0xA5, 0x80, 0x20, 0xAF, 0x02,
        0xA9, 0x20, 0x8D, 0x01, 0xF0,                            // ' '
        0xA9, 0x2B, 0x8D, 0x01, 0xF0,                            // '+'
        0xA9, 0x20, 0x8D, 0x01, 0xF0,                            // ' '
        0xA5, 0x83, 0x20, 0xAF, 0x02,
        0xA5, 0x82, 0x20, 0xAF, 0x02,
        0xA9, 0x20, 0x8D, 0x01, 0xF0,                            // ' '
        0xA9, 0x3D, 0x8D, 0x01, 0xF0,                            // '='
        0xA9, 0x20, 0x8D, 0x01, 0xF0,                            // ' '
        0xA5, 0x85, 0x20, 0xAF, 0x02,
        0xA5, 0x84, 0x20, 0xAF, 0x02,
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,                            // newline

        // SUB compute @ $025E
        0x38, 0xA5, 0x80, 0xE5, 0x82, 0x85, 0x84,                // SEC; LDA $80; SBC $82; STA $84
        0xA5, 0x81, 0xE5, 0x83, 0x85, 0x85,                      // LDA $81; SBC $83; STA $85

        // SUB print line @ $026B
        0xA5, 0x81, 0x20, 0xAF, 0x02,
        0xA5, 0x80, 0x20, 0xAF, 0x02,
        0xA9, 0x20, 0x8D, 0x01, 0xF0,
        0xA9, 0x2D, 0x8D, 0x01, 0xF0,                            // '-'
        0xA9, 0x20, 0x8D, 0x01, 0xF0,
        0xA5, 0x83, 0x20, 0xAF, 0x02,
        0xA5, 0x82, 0x20, 0xAF, 0x02,
        0xA9, 0x20, 0x8D, 0x01, 0xF0,
        0xA9, 0x3D, 0x8D, 0x01, 0xF0,
        0xA9, 0x20, 0x8D, 0x01, 0xF0,
        0xA5, 0x85, 0x20, 0xAF, 0x02,
        0xA5, 0x84, 0x20, 0xAF, 0x02,
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,

        // Halt @ $02AC
        0x4C, 0xAC, 0x02,

        // PRINT_HEX_BYTE @ $02AF
        0x48, 0x4A, 0x4A, 0x4A, 0x4A,                            // PHA; LSR A x4
        0x20, 0xBE, 0x02,                                        // JSR PRINT_NIBBLE
        0x68, 0x29, 0x0F,                                        // PLA; AND #$0F
        0x20, 0xBE, 0x02,                                        // JSR PRINT_NIBBLE
        0x60,                                                    // RTS

        // PRINT_NIBBLE @ $02BE
        0xC9, 0x0A,                                              // CMP #$0A
        0x90, 0x06,                                              // BCC IS_DIGIT
        0x18, 0x69, 0x37,                                        // CLC; ADC #$37
        0x4C, 0xCB, 0x02,                                        // JMP OUTPUT
        0x18, 0x69, 0x30,                                        // IS_DIGIT: CLC; ADC #$30
        0x8D, 0x01, 0xF0,                                        // OUTPUT: STA $F001
        0x60,                                                    // RTS
    ];

    load_and_run_program(&mut cpu, &mut bus, &multi_byte_arithmetic);
    assert_eq!(bus.output_log, b"1234 + 0089 = 12BD\n1234 - 0089 = 11AB\n");

    // The wrap-around pairs from the Expected output table.
    for (a, b, sum, difference) in [
        (0x00FF_u16, 0x0001_u16, "0100", "00FE"),
        (0x00FF, 0x00FF, "01FE", "0000"),
        (0xFF00, 0x0100, "0000", "FE00"),
        (0x0000, 0x0001, "0001", "FFFF"),
        (0x8000, 0x8000, "0000", "0000"),
    ] {
        let mut program = multi_byte_arithmetic;
        [program[1], program[5]] = a.to_le_bytes();
        [program[9], program[13]] = b.to_le_bytes();
        let mut cpu = Chip6502::new();
        let mut bus = ConsoleBus::new();
        load_and_run_program(&mut cpu, &mut bus, &program);
        let expected = format!("{a:04X} + {b:04X} = {sum}\n{a:04X} - {b:04X} = {difference}\n");
        assert_eq!(bus.output_log, expected.as_bytes());
    }
}

/// Build 14's program, 144 bytes; the first instruction sets the first divisor.
#[rustfmt::skip]
const DIVISION: [u8; 144] = [
    // Setup @ $0200
    0xA2, 0x01,                                              // LDX #1
    // LOOP @ $0202
    0x86, 0x90,                                              // STX $90
    // Print "50"
    0xA9, 0x32, 0x20, 0x72, 0x02,                            // LDA #50; JSR PRINT_DEC
    // Print " / "
    0xA9, 0x20, 0x8D, 0x01, 0xF0,
    0xA9, 0x2F, 0x8D, 0x01, 0xF0,
    0xA9, 0x20, 0x8D, 0x01, 0xF0,
    // Print divisor
    0xA5, 0x90, 0x20, 0x72, 0x02,
    // Print " = "
    0xA9, 0x20, 0x8D, 0x01, 0xF0,
    0xA9, 0x3D, 0x8D, 0x01, 0xF0,
    0xA9, 0x20, 0x8D, 0x01, 0xF0,
    // Set up DIVIDE
    0xA9, 0x32, 0x85, 0x80,                                  // dividend = 50
    0xA5, 0x90, 0x85, 0x81,                                  // divisor = saved X
    0x20, 0x5F, 0x02,                                        // JSR DIVIDE
    0x48,                                                    // PHA (save remainder)
    // Print quotient
    0xA5, 0x80, 0x20, 0x72, 0x02,
    // Print " r "
    0xA9, 0x20, 0x8D, 0x01, 0xF0,
    0xA9, 0x72, 0x8D, 0x01, 0xF0,
    0xA9, 0x20, 0x8D, 0x01, 0xF0,
    // Print remainder
    0x68, 0x20, 0x72, 0x02,                                  // PLA; JSR PRINT_DEC
    // Newline
    0xA9, 0x0A, 0x8D, 0x01, 0xF0,
    // Increment & loop
    0xA6, 0x90, 0xE8, 0xE0, 0x0A, 0xD0, 0xA6,                // LDX $90; INX; CPX #10; BNE LOOP

    // Halt @ $025C
    0x4C, 0x5C, 0x02,

    // DIVIDE @ $025F
    0xA9, 0x00, 0xA0, 0x08,                                  // LDA #0; LDY #8
    0x06, 0x80, 0x2A,                                        // ASL $80; ROL A   ← NEW: ROL A
    0xC5, 0x81, 0x90, 0x04,                                  // CMP $81; BCC DV_SKIP
    0xE5, 0x81, 0xE6, 0x80,                                  // SBC $81; INC $80
    0x88, 0xD0, 0xF2, 0x60,                                  // DEY; BNE DV_LOOP; RTS

    // PRINT_DEC @ $0272
    0xA2, 0x00,                                              // LDX #0
    0xC9, 0x0A, 0x90, 0x06, 0xE9, 0x0A, 0xE8,                // TENS_LOOP
    0x4C, 0x74, 0x02,
    0x48, 0x8A, 0xF0, 0x06,                                  // TENS_DONE
    0x18, 0x69, 0x30, 0x8D, 0x01, 0xF0,
    0x68, 0x18, 0x69, 0x30, 0x8D, 0x01, 0xF0,                // SKIP_TENS
    0x60,
];

/// Build 14 — Division via shift-and-subtract.
// Exercises, in combination:
//   2A ROL A · C5 CMP zp · A6 LDX zp · 86 STX zp
//   (plus ASL zp, SBC zp, INC zp, DEY/BNE, and Build 8's PRINT_DEC)
#[test]
#[rustfmt::skip]
fn fnl_division_via_shift_and_subtract() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    load_and_run_program(&mut cpu, &mut bus, &DIVISION);

    assert_eq!(bus.output_log, b"\
50 / 1 = 50 r 0\n\
50 / 2 = 25 r 0\n\
50 / 3 = 16 r 2\n\
50 / 4 = 12 r 2\n\
50 / 5 = 10 r 0\n\
50 / 6 = 8 r 2\n\
50 / 7 = 7 r 1\n\
50 / 8 = 6 r 2\n\
50 / 9 = 5 r 5\n");
}

/// Build 14 — the reference's short variant: first byte pair changed to
/// `LDX #9`, so one divisor runs and its trace is short enough to read.
#[test]
fn fnl_division_single_divisor() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();
    let mut program = DIVISION;
    program[1] = 0x09;

    load_and_run_program(&mut cpu, &mut bus, &program);

    assert_eq!(bus.output_log, b"50 / 9 = 5 r 5\n");
}

/// Build 15 — Bit-field / status-byte manipulation.
// Exercises, in combination:
//   09 ORA #imm · 24 BIT zp · 50 BVC rel
//   (plus AND #imm, EOR #imm, BPL, ASL zp, LDA/STA zp)
#[test]
#[rustfmt::skip]
fn fnl_bit_field_status() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    let bit_field = [
        // Setup @ $0200
        0xA9, 0x00, 0x85, 0x90, 0x20, 0x5D, 0x02,                // status = 0; print

        // SET bit 0 @ $0207
        0xA5, 0x90, 0x09, 0x01, 0x85, 0x90, 0x20, 0x5D, 0x02,

        // SET bits 7+5 @ $0210
        0xA5, 0x90, 0x09, 0xA0, 0x85, 0x90, 0x20, 0x5D, 0x02,

        // BIT test (data-ready) @ $0219
        0x24, 0x90,                                              // BIT $90
        0x10, 0x05,                                              // BPL NOT_READY1
        0xA9, 0x52, 0x4C, 0x24, 0x02,                            // LDA 'R'; JMP READY_OUT1
        0xA9, 0x2D,                                              // NOT_READY1: LDA '-'
        0x8D, 0x01, 0xF0,                                        // READY_OUT1: STA $F001
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,                            // newline

        // SET bit 6 (error) @ $022C
        0xA5, 0x90, 0x09, 0x40, 0x85, 0x90, 0x20, 0x5D, 0x02,

        // BIT test (error) @ $0235
        0x24, 0x90,                                              // BIT $90
        0x50, 0x05,                                              // BVC NO_ERROR
        0xA9, 0x45, 0x4C, 0x40, 0x02,                            // LDA 'E'; JMP ERR_OUT
        0xA9, 0x2D,                                              // NO_ERROR: LDA '-'
        0x8D, 0x01, 0xF0,                                        // ERR_OUT: STA $F001
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,                            // newline

        // CLEAR bit 6 @ $0248
        0xA5, 0x90, 0x29, 0xBF, 0x85, 0x90, 0x20, 0x5D, 0x02,

        // TOGGLE bit 7 @ $0251
        0xA5, 0x90, 0x49, 0x80, 0x85, 0x90, 0x20, 0x5D, 0x02,

        // Halt @ $025A
        0x4C, 0x5A, 0x02,

        // PRINT_STATUS @ $025D
        0xA5, 0x90, 0x85, 0xA0, 0xA0, 0x08,                      // LDA $90; STA $A0; LDY #8
        0x06, 0xA0,                                              // PS_LOOP: ASL $A0
        0xA9, 0x30,                                              // LDA #'0'
        0x90, 0x02,                                              // BCC PS_OUT
        0xA9, 0x31,                                              // LDA #'1'
        0x8D, 0x01, 0xF0,                                        // PS_OUT: STA $F001
        0x88, 0xD0, 0xF2,                                        // DEY; BNE PS_LOOP
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,                            // newline
        0x60,                                                    // RTS
    ];

    load_and_run_program(&mut cpu, &mut bus, &bit_field);

    assert_eq!(
        bus.output_log,
        b"00000000\n00000001\n10100001\nR\n11100001\nE\n10100001\n00100001\n"
    );
}

/// Build 16 — String operations (strlen, strcmp). No new opcodes.
// Exercises, in combination:
//   LDA #/zp/(zp),Y · STA zp/abs · LDX #, LDY # · INX, INY · INC zp · TXA
//   PHA, PLA · CLC, SEC · ADC # · SBC #, zp · CMP # · BCC, BEQ, BNE · JMP, JSR, RTS
#[test]
#[rustfmt::skip]
fn fnl_string_operations() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    let string_ops = [
        // ───────── Test 1: STRLEN("HELLO") @ $0200 ─────────
        0xA9, 0x15, 0x85, 0x80, 0xA9, 0x03, 0x85, 0x81,            // ptr = HELLO
        0x20, 0xAF, 0x02,                                          // JSR PRINT_STR
        0xA9, 0x20, 0x8D, 0x01, 0xF0,                              // ' '
        0x20, 0xBC, 0x02, 0x20, 0xF7, 0x02,                        // JSR STRLEN; JSR PRINT_DEC
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,                              // newline

        // ───────── Test 2: STRLEN("HELP") @ $021B ─────────
        0xA9, 0x1B, 0x85, 0x80, 0xA9, 0x03, 0x85, 0x81,            // ptr = HELP
        0x20, 0xAF, 0x02,
        0xA9, 0x20, 0x8D, 0x01, 0xF0,
        0x20, 0xBC, 0x02, 0x20, 0xF7, 0x02,
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,

        // ───────── Test 3: STRCMP(HELLO, HELP) @ $0236 ─────────
        0xA9, 0x15, 0x85, 0x80, 0xA9, 0x03, 0x85, 0x81,            // ptr = HELLO
        0x20, 0xAF, 0x02,                                          // print HELLO
        0xA9, 0x20, 0x8D, 0x01, 0xF0,                              // ' '
        0xA9, 0x1B, 0x85, 0x80, 0xA9, 0x03, 0x85, 0x81,            // ptr = HELP
        0x20, 0xAF, 0x02,                                          // print HELP
        0xA9, 0x20, 0x8D, 0x01, 0xF0,                              // ' '
        0xA9, 0x15, 0x85, 0x80, 0xA9, 0x03, 0x85, 0x81,            // $80/$81 = HELLO
        0xA9, 0x1B, 0x85, 0x82, 0xA9, 0x03, 0x85, 0x83,            // $82/$83 = HELP
        0x20, 0xCE, 0x02,                                          // JSR STRCMP
        0x8D, 0x01, 0xF0,                                          // print result
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,                              // newline

        // ───────── Test 4: STRCMP(HELLO, HELLO_2) @ $0271 ─────────
        0xA9, 0x15, 0x85, 0x80, 0xA9, 0x03, 0x85, 0x81,
        0x20, 0xAF, 0x02,
        0xA9, 0x20, 0x8D, 0x01, 0xF0,
        0xA9, 0x20, 0x85, 0x80, 0xA9, 0x03, 0x85, 0x81,            // ptr = HELLO_2
        0x20, 0xAF, 0x02,
        0xA9, 0x20, 0x8D, 0x01, 0xF0,
        0xA9, 0x15, 0x85, 0x80, 0xA9, 0x03, 0x85, 0x81,
        0xA9, 0x20, 0x85, 0x82, 0xA9, 0x03, 0x85, 0x83,            // $82/$83 = HELLO_2
        0x20, 0xCE, 0x02,
        0x8D, 0x01, 0xF0,
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,

        // ───────── Halt @ $02AC ─────────
        0x4C, 0xAC, 0x02,

        // ───────── PRINT_STR @ $02AF ─────────
        0xA0, 0x00,                                                // LDY #0
        0xB1, 0x80,                                                // PS_LOOP: LDA ($80),Y
        0xF0, 0x06,                                                // BEQ PS_DONE
        0x8D, 0x01, 0xF0,                                          // STA $F001
        0xC8, 0xD0, 0xF6,                                          // INY; BNE PS_LOOP
        0x60,                                                      // PS_DONE: RTS

        // ───────── STRLEN @ $02BC ─────────
        0xA9, 0x00, 0x85, 0x90,                                    // counter = 0
        0xA0, 0x00,                                                // LDY #0
        0xB1, 0x80,                                                // SL_LOOP: LDA ($80),Y
        0xF0, 0x05,                                                // BEQ SL_DONE
        0xE6, 0x90,                                                // INC $90
        0xC8, 0xD0, 0xF7,                                          // INY; BNE SL_LOOP
        0xA5, 0x90, 0x60,                                          // SL_DONE: LDA $90; RTS

        // ───────── STRCMP @ $02CE ─────────
        0xA0, 0x00,                                                // LDY #0
        0xB1, 0x80, 0x85, 0x90,                                    // SC_LOOP: LDA ($80),Y; STA $90
        0xB1, 0x82,                                                // LDA ($82),Y
        0xF0, 0x0B,                                                // BEQ SC_STR2_NULL
        0x38, 0xE5, 0x90,                                          // SEC; SBC $90
        0x90, 0x12,                                                // BCC SC_STR1_GREATER
        0xD0, 0x0B,                                                // BNE SC_STR1_LESS
        0xC8, 0x4C, 0xD0, 0x02,                                    // INY; JMP SC_LOOP
        0xA5, 0x90,                                                // SC_STR2_NULL: LDA $90
        0xF0, 0x0D,                                                // BEQ SC_EQUAL
        0x4C, 0xEF, 0x02,                                          // JMP SC_STR1_GREATER
        0xA9, 0x3C, 0x4C, 0xF6, 0x02,                              // SC_STR1_LESS: LDA '<'; JMP SC_DONE
        0xA9, 0x3E, 0x4C, 0xF6, 0x02,                              // SC_STR1_GREATER: LDA '>'; JMP SC_DONE
        0xA9, 0x3D,                                                // SC_EQUAL: LDA '='
        0x60,                                                      // SC_DONE: RTS

        // ───────── PRINT_DEC @ $02F7 (from Build 8) ─────────
        0xA2, 0x00,
        0xC9, 0x0A, 0x90, 0x06, 0xE9, 0x0A, 0xE8,
        0x4C, 0xF9, 0x02,
        0x48, 0x8A, 0xF0, 0x06,
        0x18, 0x69, 0x30, 0x8D, 0x01, 0xF0,
        0x68, 0x18, 0x69, 0x30, 0x8D, 0x01, 0xF0,
        0x60,

        // ───────── String data @ $0315 ─────────
        0x48, 0x45, 0x4C, 0x4C, 0x4F, 0x00,                        // "HELLO\0"  @ $0315
        0x48, 0x45, 0x4C, 0x50, 0x00,                              // "HELP\0"   @ $031B
        0x48, 0x45, 0x4C, 0x4C, 0x4F, 0x00,                        // "HELLO\0"  @ $0320
    ];

    load_and_run_program(&mut cpu, &mut bus, &string_ops);

    assert_eq!(bus.output_log, b"HELLO 5\nHELP 4\nHELLO HELP <\nHELLO HELLO =\n");
}

/// Build 17 — Memory copy (memcpy).
// Exercises, in combination:
//   91 STA (zp),Y · B1 LDA (zp),Y · DEY/BPL · STA abs
#[test]
#[rustfmt::skip]
fn fnl_memcopy() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    let memcpy_demo = [
        // ───── Print source ─────
        0xA9, 0x6D, 0x85, 0x80, 0xA9, 0x02, 0x85, 0x81,           // $80/$81 = SOURCE ($026D)
        0x20, 0x58, 0x02,                                          // JSR PRINT_STR

        // ───── MEMCPY 1: 13 bytes SOURCE → DEST1 ─────
        0xA9, 0x6D, 0x85, 0x80, 0xA9, 0x02, 0x85, 0x81,           // src = SOURCE
        0xA9, 0x00, 0x85, 0x82, 0xA9, 0x04, 0x85, 0x83,           // dst = $0400
        0xA0, 0x0C,                                                // LDY #12
        0x20, 0x65, 0x02,                                          // JSR MEMCPY

        // ───── Print DEST1 ─────
        0xA9, 0x00, 0x85, 0x80, 0xA9, 0x04, 0x85, 0x81,
        0x20, 0x58, 0x02,

        // ───── MEMCPY 2: 5 bytes SOURCE → DEST2 ─────
        0xA9, 0x6D, 0x85, 0x80, 0xA9, 0x02, 0x85, 0x81,           // src = SOURCE
        0xA9, 0x00, 0x85, 0x82, 0xA9, 0x05, 0x85, 0x83,           // dst = $0500
        0xA0, 0x04,                                                // LDY #4
        0x20, 0x65, 0x02,                                          // JSR MEMCPY

        // ───── Null-terminate DEST2 manually ─────
        0xA9, 0x00, 0x8D, 0x05, 0x05,                              // STA $0505

        // ───── Print DEST2 ─────
        0xA9, 0x00, 0x85, 0x80, 0xA9, 0x05, 0x85, 0x81,
        0x20, 0x58, 0x02,

        // ───── Manual newline + halt ─────
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,
        0x4C, 0x55, 0x02,

        // ───── PRINT_STR @ $0258 ─────
        0xA0, 0x00,                                                // LDY #0
        0xB1, 0x80,                                                // PS_LOOP: LDA ($80),Y
        0xF0, 0x06,                                                // BEQ PS_DONE
        0x8D, 0x01, 0xF0,                                          // STA $F001
        0xC8, 0xD0, 0xF6,                                          // INY; BNE PS_LOOP
        0x60,                                                      // PS_DONE: RTS

        // ───── MEMCPY @ $0265 ─────
        0xB1, 0x80,                                                // LDA ($80),Y
        0x91, 0x82,                                                // STA ($82),Y   ← NEW
        0x88,                                                      // DEY
        0x10, 0xF9,                                                // BPL MEMCPY
        0x60,                                                      // RTS

        // ───── SOURCE @ $026D ─────
        0x48, 0x45, 0x4C, 0x4C, 0x4F, 0x20, 0x57, 0x4F,            // "HELLO WO"
        0x52, 0x4C, 0x44, 0x0A, 0x00,                              // "RLD\n\0"
    ];

    load_and_run_program(&mut cpu, &mut bus, &memcpy_demo);

    assert_eq!(bus.output_log, b"HELLO WORLD\nHELLO WORLD\nHELLO\n");
}

/// Build 18 — Lookup tables (advanced indexed addressing).
// Exercises, in combination:
//   B9 LDA abs,Y · PLA/TAX · DEX/BNE · INY/CPY · Build 8's PRINT_DEC
#[test]
#[rustfmt::skip]
fn fnl_lookup_tables() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    let lookup_table = [
        // ───── Main loop @ $0200 ─────
        0xA0, 0x00,                                                // LDY #0
        0xB9, 0x45, 0x02,                                          // LDA TABLE,Y   ← NEW: LDA abs,Y
        0x48,                                                      // PHA
        0x20, 0x27, 0x02,                                          // JSR PRINT_DEC
        0xA9, 0x20, 0x8D, 0x01, 0xF0,                              // ' '
        0x68, 0xAA,                                                // PLA; TAX
        0xF0, 0x08,                                                // BEQ NEWLINE
        0xA9, 0x2A, 0x8D, 0x01, 0xF0,                              // BAR_LOOP: '*' → $F001
        0xCA,                                                      // DEX
        0xD0, 0xF8,                                                // BNE BAR_LOOP
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,                              // NEWLINE: '\n' → $F001
        0xC8,                                                      // INY
        0xC0, 0x08,                                                // CPY #8
        0xD0, 0xDE,                                                // BNE ROW_LOOP

        // ───── Halt @ $0224 ─────
        0x4C, 0x24, 0x02,

        // ───── PRINT_DEC @ $0227 (from Build 8) ─────
        0xA2, 0x00,
        0xC9, 0x0A, 0x90, 0x06, 0xE9, 0x0A, 0xE8,
        0x4C, 0x29, 0x02,
        0x48, 0x8A, 0xF0, 0x06,
        0x18, 0x69, 0x30, 0x8D, 0x01, 0xF0,
        0x68, 0x18, 0x69, 0x30, 0x8D, 0x01, 0xF0,
        0x60,

        // ───── TABLE @ $0245 ─────
        0x03, 0x07, 0x02, 0x05, 0x01, 0x04, 0x06, 0x02,
    ];

    load_and_run_program(&mut cpu, &mut bus, &lookup_table);

    assert_eq!(
        bus.output_log,
        b"3 ***\n7 *******\n2 **\n5 *****\n1 *\n4 ****\n6 ******\n2 **\n"
    );
}

/// Build 19 — Run-length decoder. No new opcodes.
// Exercises, in combination:
//   LDA (zp),Y · BEQ · TAX · INY · STA abs · DEX/BNE · JMP
#[test]
#[rustfmt::skip]
fn fnl_run_length_decoder() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    let rle_decoder = [
        // ───── Setup pointer @ $0200 ─────
        0xA9, 0x24, 0x85, 0x80,                                    // LDA #$24; STA $80
        0xA9, 0x02, 0x85, 0x81,                                    // LDA #$02; STA $81

        // ───── Decode loop @ $0208 ─────
        0xA0, 0x00,                                                // LDY #0
        0xB1, 0x80,                                                // LOOP: LDA ($80),Y
        0xF0, 0x0E,                                                // BEQ DONE
        0xAA,                                                      // TAX
        0xC8,                                                      // INY
        0xB1, 0x80,                                                // LDA ($80),Y
        0xC8,                                                      // INY
        0x8D, 0x01, 0xF0,                                          // EMIT_LOOP: STA $F001
        0xCA,                                                      // DEX
        0xD0, 0xFA,                                                // BNE EMIT_LOOP
        0x4C, 0x0A, 0x02,                                          // JMP LOOP

        // ───── DONE: emit final newline + halt @ $021C ─────
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,                              // LDA #$0A; STA $F001
        0x4C, 0x21, 0x02,                                          // JMP HALT

        // ───── ENCODED data @ $0224 ─────
        0x05, 0x23,                                                // 5 '#'
        0x01, 0x20,                                                // 1 ' '
        0x03, 0x2A,                                                // 3 '*'
        0x01, 0x20,                                                // 1 ' '
        0x05, 0x23,                                                // 5 '#'
        0x00,                                                      // end sentinel
    ];

    load_and_run_program(&mut cpu, &mut bus, &rle_decoder);

    assert_eq!(bus.output_log, b"##### *** #####\n");
}

/// Build 20 — State machine / parser. No new opcodes.
// Exercises, in combination:
//   LDA (zp),Y · PHA/PLA around the state read · LDA zp · CMP # · BEQ/BNE · INY
#[test]
#[rustfmt::skip]
fn fnl_state_machine() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    let comment_stripper = [
        // ───── Setup @ $0200 ─────
        0xA9, 0x3D, 0x85, 0x80,                                    // LDA #$3D; STA $80   (SOURCE low)
        0xA9, 0x02, 0x85, 0x81,                                    // LDA #$02; STA $81   (SOURCE high)
        0xA9, 0x00, 0x85, 0x90,                                    // state = 0 (NORMAL)
        0xA0, 0x00,                                                // LDY #0

        // ───── LOOP @ $020E ─────
        0xB1, 0x80,                                                // LDA ($80),Y
        0xF0, 0x28,                                                // BEQ DONE
        0x48,                                                      // PHA
        0xA5, 0x90,                                                // LDA $90
        0xD0, 0x12,                                                // BNE IN_COMMENT

        // ───── NORMAL @ $0217 ─────
        0x68,                                                      // PLA
        0xC9, 0x23,                                                // CMP #'#'
        0xD0, 0x07,                                                // BNE EMIT
        0xA9, 0x01, 0x85, 0x90,                                    // state = 1 (IN_COMMENT)
        0x4C, 0x37, 0x02,                                          // JMP NEXT

        // ───── EMIT @ $0223 ─────
        0x8D, 0x01, 0xF0,                                          // STA $F001
        0x4C, 0x37, 0x02,                                          // JMP NEXT

        // ───── IN_COMMENT @ $0229 ─────
        0x68,                                                      // PLA
        0xC9, 0x0A,                                                // CMP #$0A
        0xD0, 0x09,                                                // BNE NEXT
        0xA9, 0x0A, 0x8D, 0x01, 0xF0,                              // emit '\n'
        0xA9, 0x00, 0x85, 0x90,                                    // state = 0 (NORMAL)

        // ───── NEXT @ $0237 ─────
        0xC8,                                                      // INY
        0xD0, 0xD4,                                                // BNE LOOP

        // ───── DONE @ $023A ─────
        0x4C, 0x3A, 0x02,                                          // JMP DONE (halt)

        // ───── SOURCE @ $023D ─────
        0x61, 0x62, 0x63, 0x23, 0x78, 0x0A,                        // "abc#x\n"
        0x64, 0x65, 0x66, 0x23, 0x79, 0x0A,                        // "def#y\n"
        0x67, 0x68, 0x69, 0x0A,                                    // "ghi\n"
        0x00,                                                      // null terminator
    ];

    load_and_run_program(&mut cpu, &mut bus, &comment_stripper);

    assert_eq!(bus.output_log, b"abc\ndef\nghi\n");
}

/// Build 21's monitor program, 179 bytes (the same bytes Build 29's app runs).
#[rustfmt::skip]
const MONITOR: [u8; 179] = [
    // ───── Banner @ $0200 ─────
    0xA9, 0x87, 0x85, 0x80, 0xA9, 0x02, 0x85, 0x81,            // BANNER ptr
    0x20, 0x7A, 0x02,                                          // JSR PRINT_STR

    // ───── LOOP: prompt @ $020B ─────
    0xA9, 0x3E, 0x8D, 0x01, 0xF0,                              // '>'
    0xA9, 0x20, 0x8D, 0x01, 0xF0,                              // ' '

    // ───── WAIT @ $0215 ─────
    0xAD, 0x00, 0xF0,                                          // LDA $F000  ← NEW: input port read
    0xF0, 0xFB,                                                // BEQ WAIT

    // ───── Echo + newline @ $021A ─────
    0x8D, 0x01, 0xF0,                                          // STA $F001 (echo)
    0x48,                                                      // PHA
    0xA9, 0x0A, 0x8D, 0x01, 0xF0,                              // newline
    0x68,                                                      // PLA

    // ───── Dispatch @ $0224 ─────
    0xC9, 0x51, 0xF0, 0x44,                                    // CMP 'Q'; BEQ QUIT
    0xC9, 0x3F, 0xF0, 0x32,                                    // CMP '?'; BEQ HELP
    0xC9, 0x41, 0xF0, 0x12,                                    // CMP 'A'; BEQ HANDLE_A
    0xC9, 0x42, 0xF0, 0x1C,                                    // CMP 'B'; BEQ HANDLE_B

    // ───── Unknown @ $0234 ─────
    0xA9, 0xAB, 0x85, 0x80, 0xA9, 0x02, 0x85, 0x81,
    0x20, 0x7A, 0x02,
    0x4C, 0x0B, 0x02,                                          // JMP LOOP

    // ───── HANDLE_A @ $0242 ─────
    0xA9, 0x9D, 0x85, 0x80, 0xA9, 0x02, 0x85, 0x81,
    0x20, 0x7A, 0x02,
    0x4C, 0x0B, 0x02,

    // ───── HANDLE_B @ $0250 ─────
    0xA9, 0xA4, 0x85, 0x80, 0xA9, 0x02, 0x85, 0x81,
    0x20, 0x7A, 0x02,
    0x4C, 0x0B, 0x02,

    // ───── HELP @ $025E ─────
    0xA9, 0x96, 0x85, 0x80, 0xA9, 0x02, 0x85, 0x81,
    0x20, 0x7A, 0x02,
    0x4C, 0x0B, 0x02,

    // ───── QUIT @ $026C ─────
    0xA9, 0xAE, 0x85, 0x80, 0xA9, 0x02, 0x85, 0x81,
    0x20, 0x7A, 0x02,
    0x4C, 0x77, 0x02,                                          // JMP self (halt)

    // ───── PRINT_STR @ $027A ─────
    0xA0, 0x00,
    0xB1, 0x80, 0xF0, 0x06,
    0x8D, 0x01, 0xF0,
    0xC8, 0xD0, 0xF6,
    0x60,

    // ───── Strings @ $0287 ─────
    0x4D, 0x4F, 0x4E, 0x20, 0x76, 0x31, 0x0A,                  // "MON v1\n"
    0x3F, 0x3D, 0x68, 0x65, 0x6C, 0x70, 0x0A, 0x00,            // "?=help\n\0"
    0x41, 0x2C, 0x42, 0x2C, 0x51, 0x0A, 0x00,                  // "A,B,Q\n\0"
    0x67, 0x6F, 0x74, 0x20, 0x41, 0x0A, 0x00,                  // "got A\n\0"
    0x67, 0x6F, 0x74, 0x20, 0x42, 0x0A, 0x00,                  // "got B\n\0"
    0x3F, 0x0A, 0x00,                                          // "?\n\0"
    0x62, 0x79, 0x65, 0x0A, 0x00,                              // "bye\n\0"
];

/// Build 21 — Tiny interactive monitor: five keystrokes queued before the run.
// Exercises, in combination:
//   AD LDA abs (the $F000 input port) · CMP #/BEQ dispatch · PHA/PLA · PRINT_STR
#[test]
fn fnl_monitor_session() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();
    bus.synthesize_queue_input(b"?ABXQ"); // five keystrokes, queued before the run

    load_and_run_program(&mut cpu, &mut bus, &MONITOR);

    assert_eq!(
        bus.output_log,
        b"MON v1\n?=help\n> ?\nA,B,Q\n> A\ngot A\n> B\ngot B\n> X\n?\n> Q\nbye\n"
    );
}

/// Copy `program` into RAM at `address` (Build 1's loader, first half).
fn load_program(bus: &mut ConsoleBus, program: &[u8], address: u16) {
    let start = address as usize;
    bus.ram.memory[start..start + program.len()].copy_from_slice(program);
}

/// Step from the current PC to the `JMP`-self halt (Build 1's loader,
/// second half).
fn run_program(cpu: &mut Chip6502, bus: &mut ConsoleBus) {
    for _ in 0..10_000 {
        let pc_before = cpu.pc;
        cpu.step(bus);
        if cpu.pc == pc_before {
            return; // JMP-to-self: the program has halted
        }
    }
    panic!("still running after 10,000 instructions — runaway program?");
}

/// Build 24's main program at `$0200`: `CLI`, print `A`, `BRK` and its
/// padding byte, print `B` and `C`, halt.
#[rustfmt::skip]
const BRK_MAIN: [u8; 21] = [
    0x58,                       // CLI
    0xA9, 0x41,                 // LDA #'A'
    0x8D, 0x01, 0xF0,           // STA $F001
    0x00,                       // BRK
    0x00,                       // padding (BRK skip-byte)
    0xA9, 0x42,                 // LDA #'B' — RTI resumes here
    0x8D, 0x01, 0xF0,           // STA $F001
    0xA9, 0x43,                 // LDA #'C'
    0x8D, 0x01, 0xF0,           // STA $F001
    0x4C, 0x12, 0x02,           // JMP $0212 (halt)
];

/// Build 24's handler at `$0300`: print `*`, `RTI`.
#[rustfmt::skip]
const BRK_HANDLER: [u8; 6] = [
    0xA9, 0x2A,                 // LDA #'*'
    0x8D, 0x01, 0xF0,           // STA $F001
    0x40,                       // RTI
];

/// Build 24 — Interrupts (BRK round-trip).
// Exercises, in combination:
//   00 BRK (entry, the +2 skip byte) · 40 RTI · 58 CLI · the $FFFE vector
#[test]
#[rustfmt::skip]
fn fnl_brk_round_trip() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    // Two blocks, two addresses.
    load_program(&mut bus, &BRK_MAIN, 0x0200);
    load_program(&mut bus, &BRK_HANDLER, 0x0300);

    // The vector poke no earlier test needed. Without it, BRK
    // jumps to $0000, where zeroed memory holds another BRK; that
    // one jumps to $0000 again, run_program sees PC unchanged and
    // stops, and the assert fails with only "A" printed.
    bus.ram.memory[0xFFFE] = 0x00;  bus.ram.memory[0xFFFF] = 0x03; // IRQ/BRK → $0300

    cpu.pc = 0x0200;

    run_program(&mut cpu, &mut bus);

    assert_eq!(
        bus.output_log, b"A*BC",
        "interrupt round-trip failed: expected 'A*BC', got {:?}",
        String::from_utf8_lossy(&bus.output_log)
    );
}

/// Build 24's extension main program: `CLI`, print `A`, `NOP` (the harness
/// raises the interrupt here), print `D`, halt.
#[rustfmt::skip]
const EXTENSION_MAIN: [u8; 15] = [
    0x58,                       // CLI
    0xA9, 0x41, 0x8D, 0x01, 0xF0, // LDA #'A'; STA $F001
    0xEA,                       // WAIT: NOP  ($0206)
    0xA9, 0x44, 0x8D, 0x01, 0xF0, // LDA #'D'; STA $F001
    0x4C, 0x0C, 0x02,           // DONE: JMP $020C
];

/// Run to the halt, calling `poke` before every step so the test can
/// drive the interrupt lines.
fn run_with(cpu: &mut Chip6502, bus: &mut ConsoleBus, mut poke: impl FnMut(&mut Chip6502)) {
    for _ in 0..200 {
        poke(cpu);
        let pc_before = cpu.pc;
        cpu.step(bus);
        if cpu.pc == pc_before {
            return;
        }
    }
    panic!("still running after 200 steps");
}

/// Build 24, Extension A — a hardware IRQ round trip. The handler releases
/// nothing itself, so the harness clears the line once the handler runs.
#[test]
fn fnl_irq_round_trip() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();
    load_program(&mut bus, &EXTENSION_MAIN, 0x0200);
    load_program(&mut bus, &BRK_HANDLER, 0x0300);
    bus.ram.memory[0xFFFE..=0xFFFF].copy_from_slice(&[0x00, 0x03]);
    cpu.pc = 0x0200;

    let mut irq_fired = false;
    run_with(&mut cpu, &mut bus, |cpu| {
        if !irq_fired && cpu.pc == 0x0206 {
            cpu.request_irq();
            irq_fired = true;
        }
        if irq_fired && (0x0300..0x0306).contains(&cpu.pc) {
            cpu.clear_irq(); // level-triggered: acknowledge, or it fires again
        }
    });

    assert_eq!(bus.output_log, b"A*D");
}

/// Build 24, Extension B — an NMI round trip, with interrupts masked to
/// show that I does not stop it.
#[test]
fn fnl_nmi_round_trip() {
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();
    let mut main = EXTENSION_MAIN;
    main[0] = 0x78; // SEI in place of CLI: NMI ignores the mask
    load_program(&mut bus, &main, 0x0200);
    load_program(&mut bus, &BRK_HANDLER, 0x0400);
    bus.ram.memory[0xFFFA..=0xFFFB].copy_from_slice(&[0x00, 0x04]);
    cpu.pc = 0x0200;

    let mut nmi_fired = false;
    run_with(&mut cpu, &mut bus, |cpu| {
        if !nmi_fired && cpu.pc == 0x0206 {
            cpu.trigger_nmi(); // edge-triggered: no clear needed
            nmi_fired = true;
        }
    });

    assert_eq!(bus.output_log, b"A*D");
}

/// Build 24, Extension C — an NMI arrives inside an IRQ handler; both
/// frames unwind in order.
#[test]
#[rustfmt::skip]
fn fnl_nested_interrupts() {
    let irq_handler = [
        0xA9, 0x5B, 0x8D, 0x01, 0xF0, // LDA #'['; STA $F001  ($0300)
        0xEA, 0xEA,                   // NOP; NOP             ($0305, NMI arrives here)
        0xA9, 0x5D, 0x8D, 0x01, 0xF0, // LDA #']'; STA $F001
        0x40,                         // RTI
    ];
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();
    load_program(&mut bus, &EXTENSION_MAIN, 0x0200);
    load_program(&mut bus, &irq_handler, 0x0300);
    load_program(&mut bus, &BRK_HANDLER, 0x0400); // NMI handler prints '*'
    bus.ram.memory[0xFFFA..=0xFFFF].copy_from_slice(&[0x00, 0x04, 0x00, 0x00, 0x00, 0x03]);
    cpu.pc = 0x0200;

    let (mut irq_fired, mut nmi_fired) = (false, false);
    run_with(&mut cpu, &mut bus, |cpu| {
        if !irq_fired && cpu.pc == 0x0206 {
            cpu.request_irq();
            irq_fired = true;
        }
        if irq_fired && cpu.pc == 0x0300 {
            cpu.clear_irq();
        }
        if !nmi_fired && cpu.pc == 0x0305 {
            cpu.trigger_nmi();
            nmi_fired = true;
        }
    });

    assert_eq!(bus.output_log, b"A[*]D");
}

/// Build 24, Extension D — a handler that reads the marker byte after
/// `BRK` through the pushed return address, and prints `W` + marker.
#[test]
#[rustfmt::skip]
fn fnl_brk_marker_dispatch() {
    let main = [
        0x00, 0x01,                   // BRK; .byte $01
        0x00, 0x02,                   // BRK; .byte $02
        0x4C, 0x04, 0x02,             // JMP $0204
    ];
    let handler = [
        0xBA,                         // TSX
        0xBD, 0x02, 0x01,             // LDA $0102,X   pushed PCL
        0x38, 0xE9, 0x01,             // SEC; SBC #1
        0x85, 0x80,                   // STA $80       marker pointer low
        0xBD, 0x03, 0x01,             // LDA $0103,X   pushed PCH
        0xE9, 0x00,                   // SBC #0        subtract borrow
        0x85, 0x81,                   // STA $81
        0xA0, 0x00,                   // LDY #0
        0xB1, 0x80,                   // LDA ($80),Y   A = BRK marker byte
        0x18, 0x69, 0x57,             // CLC; ADC #'W' — marker 1 → 'X', 2 → 'Y'
        0x8D, 0x01, 0xF0,             // STA $F001
        0x40,                         // RTI
    ];
    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();
    load_program(&mut bus, &main, 0x0200);
    load_program(&mut bus, &handler, 0x0300);
    bus.ram.memory[0xFFFE..=0xFFFF].copy_from_slice(&[0x00, 0x03]);
    cpu.pc = 0x0200;
    cpu.sp = 0xFF;

    run_program(&mut cpu, &mut bus);

    assert_eq!(bus.output_log, b"XY");
}

/// Build 25 — BCD mode (decimal-mode ADC/SBC).
// Exercises, in combination:
//   F8 SED · D8 CLD · 69 ADC #imm and E9 SBC #imm with D set · 08 PHP → 68 PLA
//   (plus ORA #imm, AND #imm, LSR A, JSR/RTS)
#[test]
#[rustfmt::skip]
fn fnl_bcd_mode() {
    let program = [
        // === main at $0200 ===
        0xF8,                         // SED
        0xA9, 0x25,                   // LDA #$25
        0x18,                         // CLC
        0x69, 0x37,                   // ADC #$37
        0x20, 0x3F, 0x02,             // JSR PRINT_BCD ($023F)
        0x20, 0x52, 0x02,             // JSR PRINT_NL  ($0252)
        0xA9, 0x27,                   // LDA #$27
        0x18,                         // CLC
        0x69, 0x16,                   // ADC #$16
        0x20, 0x3F, 0x02,             // JSR PRINT_BCD
        0x20, 0x52, 0x02,             // JSR PRINT_NL
        0xA9, 0x99,                   // LDA #$99
        0x18,                         // CLC
        0x69, 0x01,                   // ADC #$01
        0x08,                         // PHP
        0x20, 0x3F, 0x02,             // JSR PRINT_BCD
        0x20, 0x52, 0x02,             // JSR PRINT_NL
        0x68,                         // PLA
        0x29, 0x01,                   // AND #$01
        0x09, 0x30,                   // ORA #$30
        0x8D, 0x01, 0xF0,             // STA $F001
        0x20, 0x52, 0x02,             // JSR PRINT_NL
        0xA9, 0x50,                   // LDA #$50
        0x38,                         // SEC
        0xE9, 0x25,                   // SBC #$25
        0x20, 0x3F, 0x02,             // JSR PRINT_BCD
        0x20, 0x52, 0x02,             // JSR PRINT_NL
        0xD8,                         // CLD
        0x4C, 0x3A, 0x02,             // JMP $023A (DONE: halt)
    ];

    let subroutines = [
        // === PRINT_BCD at $023F (19 bytes) ===
        0x48,                         // PHA
        0x4A, 0x4A, 0x4A, 0x4A,       // LSR A x4
        0x09, 0x30,                   // ORA #$30
        0x8D, 0x01, 0xF0,             // STA $F001
        0x68,                         // PLA
        0x29, 0x0F,                   // AND #$0F
        0x09, 0x30,                   // ORA #$30
        0x8D, 0x01, 0xF0,             // STA $F001
        0x60,                         // RTS

        // === PRINT_NL at $0252 (6 bytes) ===
        0xA9, 0x0A,                   // LDA #$0A
        0x8D, 0x01, 0xF0,             // STA $F001
        0x60,                         // RTS
    ];

    let mut cpu = Chip6502::new();
    let mut bus = ConsoleBus::new();

    load_program(&mut bus, &program, 0x0200);
    load_program(&mut bus, &subroutines, 0x023F);
    cpu.pc = 0x0200;

    run_program(&mut cpu, &mut bus);

    assert_eq!(
        bus.output_log, b"62\n43\n00\n1\n25\n",
        "BCD test failed: expected '62\\n43\\n00\\n1\\n25\\n', got {:?}",
        String::from_utf8_lossy(&bus.output_log)
    );
}
