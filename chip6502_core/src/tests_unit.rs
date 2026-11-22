// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! Unit tests: one instruction per test, every flag pinned.
//!
//! Each test places an instruction at `cpu.pc` on a `FlatBus`, runs one
//! `step`, and asserts the result, where PC landed, and the full NVZC state
//! through `assert_flags`, with a decoy flag planted to prove that "others
//! unchanged" holds.
//!
//! Introduced in Chapter 5 — The Method and the Map (the Opcode Development Loop);
//! Build 1 — Hello, world adds `assert_flags`, and each build adds the rows its instruction table names.

use crate::{Chip6502, FlatBus, StatusFlag};

/// Assert N, V, Z and C together, so a failure prints the whole flag state.
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "Build 1's printed helper takes the four flags as booleans"
)]
#[expect(
    clippy::uninlined_format_args,
    reason = "Build 1's printed helper uses positional format arguments"
)]
#[rustfmt::skip]
fn assert_flags(
    cpu: &Chip6502,
    expected_n: bool,
    expected_v: bool,
    expected_z: bool,
    expected_c: bool,
    msg: &str,
) {
    let n = cpu.p & StatusFlag::Negative as u8 != 0;
    let v = cpu.p & StatusFlag::Overflow as u8 != 0;
    let z = cpu.p & StatusFlag::Zero as u8 != 0;
    let c = cpu.p & StatusFlag::Carry as u8 != 0;
    assert_eq!(
        (n, v, z, c),
        (expected_n, expected_v, expected_z, expected_c),
        "{}: NVZC expected ({},{},{},{}), got ({},{},{},{})",
        msg, expected_n, expected_v, expected_z, expected_c, n, v, z, c
    );
}

// ----- Chapter 5 walkthrough and Build 1: Hello, world -----

#[test]
#[rustfmt::skip]
fn ldx_immediate_loads_x_and_sets_flags() {
    let mut bus = FlatBus::new();
    let mut cpu = Chip6502::new();
    cpu.pc = 0x0200;
    cpu.p |= StatusFlag::Carry as u8;  // set C — LDX must leave it alone

    bus.memory[0x0200] = 0xA2;         // LDX #$44
    bus.memory[0x0201] = 0x44;
    cpu.step(&mut bus);
    assert_eq!(cpu.x, 0x44);
    assert_eq!(cpu.pc, 0x0202);        // stepped past both bytes
    assert_flags(&cpu, false, false, false, true, "LDX #$44");

    bus.memory[0x0202] = 0xA2;         // LDX #$00 — the zero sets Z
    bus.memory[0x0203] = 0x00;
    cpu.step(&mut bus);
    assert_eq!(cpu.x, 0x00);
    assert_flags(&cpu, false, false, true, true, "LDX #$00");

    bus.memory[0x0204] = 0xA2;         // LDX #$80 — bit 7 sets N
    bus.memory[0x0205] = 0x80;
    cpu.step(&mut bus);
    assert_eq!(cpu.x, 0x80);
    assert_flags(&cpu, true, false, false, true, "LDX #$80");
}

#[test]
fn lda_immediate_loads_a_and_sets_flags() {
    let mut bus = FlatBus::new();
    let mut cpu = Chip6502::new();
    cpu.pc = 0x0200;
    cpu.p |= StatusFlag::Carry as u8;
    bus.memory[0x0200..0x0206].copy_from_slice(&[0xA9, 0x44, 0xA9, 0x00, 0xA9, 0x80]);

    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x44);
    assert_eq!(cpu.pc, 0x0202);
    assert_flags(&cpu, false, false, false, true, "LDA #$44");

    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00);
    assert_flags(&cpu, false, false, true, true, "LDA #$00");

    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x80);
    assert_flags(&cpu, true, false, false, true, "LDA #$80");
}

#[test]
#[rustfmt::skip]
fn lda_absolute_x_indexes_from_base() {
    let mut bus = FlatBus::new();
    let mut cpu = Chip6502::new();
    bus.memory[0x0200] = 0xBD;         // LDA $0300,X
    bus.memory[0x0201] = 0x00;
    bus.memory[0x0202] = 0x03;
    bus.memory[0x0300] = 0xEE;         // decoy — the answer if X is ignored
    bus.memory[0x0305] = 0x42;         // the answer if X is added
    cpu.pc = 0x0200;
    cpu.x = 0x05;

    cpu.step(&mut bus);

    assert_eq!(cpu.a, 0x42);
    assert_eq!(cpu.pc, 0x0203);        // stepped past all three bytes
}

#[test]
fn lda_absolute_x_sets_flags() {
    let mut bus = FlatBus::new();
    let mut cpu = Chip6502::new();
    cpu.p |= StatusFlag::Carry as u8;
    cpu.x = 0x01;
    bus.memory[0x0300] = 0x55; // decoy at the unindexed base
    bus.memory[0x0301] = 0x00;
    bus.memory[0x0302] = 0x80;
    bus.memory[0x0200..0x0206].copy_from_slice(&[0xBD, 0x00, 0x03, 0xBD, 0x01, 0x03]);
    cpu.pc = 0x0200;

    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00);
    assert_flags(&cpu, false, false, true, true, "LDA $0300,X -> $00");

    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x80);
    assert_flags(&cpu, true, false, false, true, "LDA $0301,X -> $80");
}

#[test]
fn sta_absolute_stores_a() {
    let mut bus = FlatBus::new();
    let mut cpu = Chip6502::new();
    cpu.pc = 0x0200;
    cpu.a = 0x5A;
    bus.memory[0x0200..0x0203].copy_from_slice(&[0x8D, 0x34, 0x12]); // STA $1234

    cpu.step(&mut bus);

    assert_eq!(bus.memory[0x1234], 0x5A);
    assert_eq!(cpu.pc, 0x0203);
}

#[test]
fn inx_increments_x_and_sets_flags() {
    let mut bus = FlatBus::new();
    let mut cpu = Chip6502::new();
    cpu.p |= StatusFlag::Carry as u8; // INX never touches C
    bus.memory[0x0200..0x0203].copy_from_slice(&[0xE8, 0xE8, 0xE8]);
    cpu.pc = 0x0200;

    cpu.x = 0x41;
    cpu.step(&mut bus);
    assert_eq!(cpu.x, 0x42);
    assert_eq!(cpu.pc, 0x0201);
    assert_flags(&cpu, false, false, false, true, "INX $41");

    cpu.x = 0xFF;
    cpu.step(&mut bus);
    assert_eq!(cpu.x, 0x00);
    assert_flags(&cpu, false, false, true, true, "INX $FF wraps");

    cpu.x = 0x7F;
    cpu.step(&mut bus);
    assert_eq!(cpu.x, 0x80);
    assert_flags(&cpu, true, false, false, true, "INX $7F");
}

#[test]
fn beq_taken_and_not_taken() {
    let mut bus = FlatBus::new();
    let mut cpu = Chip6502::new();
    bus.memory[0x0205] = 0xF0; // BEQ +6
    bus.memory[0x0206] = 0x06;

    cpu.pc = 0x0205;
    cpu.p = StatusFlag::Zero as u8;
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x020D, "taken: $0205 + 2 + 6");

    cpu.pc = 0x0205;
    cpu.p = 0;
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0207, "not taken: steps past both bytes");
}

#[test]
fn bne_taken_and_not_taken() {
    let mut bus = FlatBus::new();
    let mut cpu = Chip6502::new();
    bus.memory[0x020B] = 0xD0; // BNE -11
    bus.memory[0x020C] = 0xF5;

    cpu.pc = 0x020B;
    cpu.p = StatusFlag::Carry as u8;
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0202, "taken backwards: $020B + 2 - 11");
    assert_flags(&cpu, false, false, false, true, "BNE moves no flag");

    cpu.pc = 0x020B;
    cpu.p = StatusFlag::Zero as u8;
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x020D, "not taken");
}

#[test]
fn jmp_absolute_sets_pc() {
    let mut bus = FlatBus::new();
    let mut cpu = Chip6502::new();
    cpu.pc = 0x0200;
    bus.memory[0x0200..0x0203].copy_from_slice(&[0x4C, 0x34, 0x12]); // JMP $1234

    cpu.step(&mut bus);

    assert_eq!(cpu.pc, 0x1234);
}

// ----- Build 2: Count 0 to 9 with subroutines -----

#[test]
fn txa_transfers_x_and_sets_flags() {
    let mut bus = FlatBus::new();
    let mut cpu = Chip6502::new();
    cpu.p |= StatusFlag::Carry as u8;
    bus.memory[0x0200..0x0203].copy_from_slice(&[0x8A, 0x8A, 0x8A]);
    cpu.pc = 0x0200;

    cpu.x = 0x44;
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x44);
    assert_eq!(cpu.pc, 0x0201);
    assert_flags(&cpu, false, false, false, true, "TXA $44");

    cpu.x = 0x00;
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00);
    assert_flags(&cpu, false, false, true, true, "TXA $00");

    cpu.x = 0x80;
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x80);
    assert_flags(&cpu, true, false, false, true, "TXA $80");
}

#[test]
fn clc_clears_carry() {
    let mut bus = FlatBus::new();
    let mut cpu = Chip6502::new();
    cpu.p = StatusFlag::Carry as u8 | StatusFlag::Zero as u8; // Z is the decoy
    bus.memory[0x0200] = 0x18;
    cpu.pc = 0x0200;

    cpu.step(&mut bus);

    assert_eq!(cpu.pc, 0x0201);
    assert_flags(&cpu, false, false, true, false, "CLC");
}

/// ADC and SBC as data: one case table per instruction, one runner, and
/// one function per addressing mode that plants the operand where that
/// mode reads it (Builds 2, 8, 10 and 13).
mod arithmetic_operations {
    use super::assert_flags;
    use crate::{Chip6502, FlatBus, StatusFlag};

    /// One arithmetic case: the inputs, the expected A, and the expected
    /// flags written as a pattern such as `"N..C"` (letters set, dots clear).
    struct TestCase {
        a: u8,
        operand: u8,
        carry_in: bool,
        result: u8,
        nvzc: &'static str,
        note: &'static str,
    }

    /// Run every case for one opcode, setting up the operand with `setup`.
    fn run_cases(opcode: u8, length: u16, cases: &[TestCase], setup: fn(&mut FlatBus, u8)) {
        for case in cases {
            let mut bus = FlatBus::new();
            let mut cpu = Chip6502::new();
            cpu.a = case.a;
            if case.carry_in {
                cpu.p |= StatusFlag::Carry as u8;
            }
            bus.memory[0x0200] = opcode;
            setup(&mut bus, case.operand);
            cpu.pc = 0x0200;

            cpu.step(&mut bus);

            let msg = format!("opcode ${opcode:02X}: {}", case.note);
            assert_eq!(cpu.a, case.result, "{msg}");
            assert_eq!(cpu.pc, 0x0200 + length, "{msg}");
            let flag = |letter| case.nvzc.contains(letter);
            assert_flags(&cpu, flag('N'), flag('V'), flag('Z'), flag('C'), &msg);
        }
    }

    /// Immediate: the operand is the byte after the opcode.
    fn immediate(bus: &mut FlatBus, operand: u8) {
        bus.memory[0x0201] = operand;
    }

    #[rustfmt::skip]
    const ADC_CASES: [TestCase; 7] = [
        TestCase { a: 0x10, operand: 0x20, carry_in: false, result: 0x30, nvzc: "....", note: "plain add" },
        TestCase { a: 0x10, operand: 0x20, carry_in: true,  result: 0x31, nvzc: "....", note: "carry-in adds one" },
        TestCase { a: 0xFF, operand: 0x01, carry_in: false, result: 0x00, nvzc: "..ZC", note: "wraps to zero, C out" },
        TestCase { a: 0x50, operand: 0x50, carry_in: false, result: 0xA0, nvzc: "NV..", note: "+80 + +80 overflows" },
        TestCase { a: 0xD0, operand: 0x90, carry_in: false, result: 0x60, nvzc: ".V.C", note: "-48 + -112 overflows" },
        TestCase { a: 0x80, operand: 0x01, carry_in: false, result: 0x81, nvzc: "N...", note: "N from the result" },
        TestCase { a: 0xFF, operand: 0xFF, carry_in: true,  result: 0xFF, nvzc: "N..C", note: "carry out with carry in" },
    ];

    /// Zero page: the byte after the opcode is an address, and the operand
    /// lives there. The address (`$90`) and the operands differ, so a handler
    /// that confuses the two fails.
    fn zero_page(bus: &mut FlatBus, operand: u8) {
        bus.memory[0x0201] = 0x90;
        bus.memory[0x0090] = operand;
    }

    #[rustfmt::skip]
    const SBC_CASES: [TestCase; 7] = [
        TestCase { a: 0x50, operand: 0x30, carry_in: true,  result: 0x20, nvzc: "...C", note: "clean subtract" },
        TestCase { a: 0x50, operand: 0x30, carry_in: false, result: 0x1F, nvzc: "...C", note: "incoming borrow takes one more" },
        TestCase { a: 0x30, operand: 0x50, carry_in: true,  result: 0xE0, nvzc: "N...", note: "borrow out" },
        TestCase { a: 0x30, operand: 0x30, carry_in: true,  result: 0x00, nvzc: "..ZC", note: "exact zero" },
        TestCase { a: 0x50, operand: 0xB0, carry_in: true,  result: 0xA0, nvzc: "NV..", note: "+80 - -80 overflows" },
        TestCase { a: 0xD0, operand: 0x70, carry_in: true,  result: 0x60, nvzc: ".V.C", note: "-48 - +112 overflows" },
        TestCase { a: 0x80, operand: 0x00, carry_in: false, result: 0x7F, nvzc: ".V.C", note: "the borrow alone crosses the signed edge" },
    ];

    #[test]
    fn adc_immediate() {
        run_cases(0x69, 2, &ADC_CASES, immediate);
    }

    #[test]
    fn adc_zero_page() {
        run_cases(0x65, 2, &ADC_CASES, zero_page);
    }

    #[test]
    fn sbc_immediate() {
        run_cases(0xE9, 2, &SBC_CASES, immediate);
    }

    #[test]
    fn sbc_zero_page() {
        run_cases(0xE5, 2, &SBC_CASES, zero_page);
    }

    // ----- Build 22: the remaining modes, each only an operand test setup function -----

    /// Zero page,X with X = 0: the base `$90` holds the operand.
    fn zero_page_x(bus: &mut FlatBus, operand: u8) {
        bus.memory[0x0201] = 0x90;
        bus.memory[0x0090] = operand;
    }

    /// Absolute: `$1234`.
    fn absolute(bus: &mut FlatBus, operand: u8) {
        bus.memory[0x0201..0x0203].copy_from_slice(&[0x34, 0x12]);
        bus.memory[0x1234] = operand;
    }

    /// `(zp,X)` with X = 0: pointer at `$40` → `$1234`.
    fn indexed_indirect(bus: &mut FlatBus, operand: u8) {
        bus.memory[0x0201] = 0x40;
        bus.memory[0x0040..0x0042].copy_from_slice(&[0x34, 0x12]);
        bus.memory[0x1234] = operand;
    }

    /// `(zp),Y` with Y = 0: pointer at `$40` → `$1234`.
    fn indirect_indexed(bus: &mut FlatBus, operand: u8) {
        indexed_indirect(bus, operand);
    }

    #[test]
    fn adc_and_sbc_every_mode() {
        // abs,X and abs,Y read at base + 0 here; the index arithmetic is
        // pinned by the addressing_modes tests.
        for (adc, sbc, length, setup) in [
            (0x75, 0xF5, 2, zero_page_x as fn(&mut FlatBus, u8)),
            (0x6D, 0xED, 3, absolute),
            (0x7D, 0xFD, 3, absolute),
            (0x79, 0xF9, 3, absolute),
            (0x61, 0xE1, 2, indexed_indirect),
            (0x71, 0xF1, 2, indirect_indexed),
        ] {
            run_cases(adc, length, &ADC_CASES, setup);
            run_cases(sbc, length, &SBC_CASES, setup);
        }
    }
}

/// Build 22: one test per addressing mode, exercising the arithmetic
/// `resolve_addr` owns, with the page-zero wraps as the important cases.
mod addressing_modes {
    use super::setup;

    #[test]
    fn zero_page_x_wraps_within_page_zero() {
        let (mut cpu, mut bus) = setup(&[0xB5, 0x80]); // LDA $80,X
        cpu.x = 0xA0;
        bus.memory[0x0020] = 0x42;
        bus.memory[0x0120] = 0x99; // read if the add escaped page zero
        cpu.step(&mut bus);
        assert_eq!((cpu.a, cpu.pc), (0x42, 0x0202));
    }

    #[test]
    fn zero_page_y_wraps_within_page_zero() {
        let (mut cpu, mut bus) = setup(&[0xB6, 0xFF]); // LDX $FF,Y
        cpu.y = 0x02;
        bus.memory[0x0001] = 0x42;
        bus.memory[0x0101] = 0x99;
        cpu.step(&mut bus);
        assert_eq!((cpu.x, cpu.pc), (0x42, 0x0202));
    }

    #[test]
    fn absolute_x_crosses_a_page() {
        let (mut cpu, mut bus) = setup(&[0xBD, 0xFF, 0x12]); // LDA $12FF,X
        cpu.x = 0x01;
        bus.memory[0x1300] = 0x42;
        bus.memory[0x1200] = 0x99; // read if the carry into the high byte were lost
        cpu.step(&mut bus);
        assert_eq!((cpu.a, cpu.pc), (0x42, 0x0203));
        assert!(cpu.page_crossed, "absolute,X records the crossing");
    }

    #[test]
    fn absolute_y_adds_y() {
        let (mut cpu, mut bus) = setup(&[0xB9, 0xF0, 0x12]); // LDA $12F0,Y
        cpu.y = 0x20;
        bus.memory[0x1310] = 0x42;
        cpu.step(&mut bus);
        assert_eq!((cpu.a, cpu.pc), (0x42, 0x0203));
    }

    #[test]
    fn indexed_indirect_adds_x_before_reading_the_pointer() {
        let (mut cpu, mut bus) = setup(&[0xA1, 0x40, 0xA1, 0xFE]); // LDA ($40,X); LDA ($FE,X)
        cpu.x = 0x04;
        bus.memory[0x0044..0x0046].copy_from_slice(&[0x00, 0x03]); // ($40 + 4) → $0300
        bus.memory[0x0040..0x0042].copy_from_slice(&[0x00, 0x04]); // decoy if X were ignored
        bus.memory[0x0300] = 0x42;
        cpu.step(&mut bus);
        assert_eq!((cpu.a, cpu.pc), (0x42, 0x0202));

        // $FE + 4 = $02 (wraps), and a pointer at $FF reads its high byte from $00.
        cpu.x = 0x01;
        bus.memory[0x00FF] = 0x10;
        bus.memory[0x0000] = 0x05;
        bus.memory[0x0100] = 0x06; // read if the high-byte fetch escaped page zero
        bus.memory[0x0510] = 0x77;
        cpu.step(&mut bus);
        assert_eq!(cpu.a, 0x77);
    }

    #[test]
    fn stores_use_every_mode() {
        let (mut cpu, mut bus) = setup(&[
            0x95, 0x80, // STA $80,X
            0x9D, 0x00, 0x03, // STA $0300,X
            0x99, 0x00, 0x03, // STA $0300,Y
            0x81, 0x40, // STA ($40,X)
            0x96, 0x80, // STX $80,Y
            0x8E, 0x00, 0x05, // STX $0500
            0x94, 0xA0, // STY $A0,X
            0x8C, 0x01, 0x05, // STY $0501
            0x84, 0x90, // STY $90
        ]);
        cpu.a = 0xAA;
        cpu.x = 0x02;
        cpu.y = 0x03;
        bus.memory[0x0042..0x0044].copy_from_slice(&[0x00, 0x06]);
        for _ in 0..9 {
            cpu.step(&mut bus);
        }
        assert_eq!(bus.memory[0x0082], 0xAA, "STA zp,X");
        assert_eq!(bus.memory[0x0302], 0xAA, "STA abs,X");
        assert_eq!(bus.memory[0x0303], 0xAA, "STA abs,Y");
        assert_eq!(bus.memory[0x0600], 0xAA, "STA (zp,X)");
        assert_eq!(bus.memory[0x0083], 0x02, "STX zp,Y");
        assert_eq!(bus.memory[0x0500], 0x02, "STX abs");
        assert_eq!(bus.memory[0x0501], 0x03, "STY abs");
        assert_eq!(bus.memory[0x0090], 0x03, "STY zp");
        assert_eq!(bus.memory[0x00A2], 0x03, "STY zp,X");
        assert_eq!(cpu.pc, 0x0216);
    }
}

/// Build 22: the twins and the instructions new in the mop-up.
mod mop_up {
    use super::{assert_flags, setup};
    use crate::StatusFlag;

    #[test]
    fn tay_tya_transfer_and_set_flags() {
        let (mut cpu, mut bus) = setup(&[0xA8, 0x98]);
        cpu.p = StatusFlag::Carry as u8;
        cpu.a = 0x80;
        cpu.step(&mut bus);
        assert_eq!(cpu.y, 0x80);
        assert_flags(&cpu, true, false, false, true, "TAY $80");
        cpu.y = 0x00;
        cpu.step(&mut bus);
        assert_eq!((cpu.a, cpu.pc), (0x00, 0x0202));
        assert_flags(&cpu, false, false, true, true, "TYA $00");
    }

    #[test]
    fn tsx_sets_flags_and_txs_does_not() {
        let (mut cpu, mut bus) = setup(&[0xBA, 0x9A]);
        cpu.sp = 0xF0;
        cpu.step(&mut bus);
        assert_eq!(cpu.x, 0xF0);
        assert_flags(&cpu, true, false, false, false, "TSX $F0");
        cpu.x = 0x00;
        cpu.p = 0;
        cpu.step(&mut bus);
        assert_eq!((cpu.sp, cpu.pc), (0x00, 0x0202));
        assert_flags(
            &cpu,
            false,
            false,
            false,
            false,
            "TXS sets no flags, even for zero",
        );
    }

    #[test]
    fn php_pushes_b_and_u_and_plp_masks_b() {
        let (mut cpu, mut bus) = setup(&[0x08, 0x28]);
        cpu.sp = 0xFF;
        cpu.p = StatusFlag::Carry as u8;
        cpu.step(&mut bus);
        assert_eq!(bus.memory[0x01FF], 0x31, "PHP pushes P | B | U");
        assert_eq!(cpu.p, StatusFlag::Carry as u8, "the live P is untouched");

        bus.memory[0x01FF] = 0xFF; // plant every bit, B included
        cpu.step(&mut bus);
        assert_eq!(cpu.p, 0xEF, "PLP masks B off and forces U on");
        assert_eq!((cpu.sp, cpu.pc), (0xFF, 0x0202));
    }

    #[test]
    fn bcs_bmi_bvs_branch_on_set_flags() {
        for (opcode, flag) in [
            (0xB0, StatusFlag::Carry),
            (0x30, StatusFlag::Negative),
            (0x70, StatusFlag::Overflow),
        ] {
            let (mut cpu, mut bus) = setup(&[opcode, 0x10]);
            cpu.step(&mut bus);
            assert_eq!(
                cpu.pc, 0x0202,
                "${opcode:02X} not taken when the flag is clear"
            );
            cpu.pc = 0x0200;
            cpu.p = flag as u8;
            cpu.step(&mut bus);
            assert_eq!(cpu.pc, 0x0212, "${opcode:02X} taken when the flag is set");
        }
    }

    #[test]
    fn nop_only_advances_pc() {
        let (mut cpu, mut bus) = setup(&[0xEA]);
        cpu.p = 0xFF;
        cpu.a = 0x12;
        cpu.step(&mut bus);
        assert_eq!((cpu.pc, cpu.p, cpu.a), (0x0201, 0xFF, 0x12));
    }

    #[test]
    fn ror_accumulator_and_memory() {
        let (mut cpu, mut bus) = setup(&[0x6A, 0x6A, 0x66, 0x90]);
        cpu.p = StatusFlag::Carry as u8;
        cpu.a = 0x02;
        cpu.step(&mut bus);
        assert_eq!(cpu.a, 0x81, "C enters at bit 7");
        assert_flags(&cpu, true, false, false, false, "ROR $02, C=1");
        cpu.a = 0x01;
        cpu.step(&mut bus);
        assert_eq!(cpu.a, 0x00);
        assert_flags(&cpu, false, false, true, true, "ROR $01, C=0");
        bus.memory[0x0090] = 0x00; // C is 1: becomes $80
        cpu.step(&mut bus);
        assert_eq!((bus.memory[0x0090], cpu.pc), (0x80, 0x0204));
        assert_flags(&cpu, true, false, false, false, "ROR $90");
    }

    #[test]
    fn lsr_memory_clears_n() {
        let (mut cpu, mut bus) = setup(&[0x4E, 0x34, 0x12]);
        cpu.p = StatusFlag::Negative as u8;
        bus.memory[0x1234] = 0x81;
        cpu.step(&mut bus);
        assert_eq!((bus.memory[0x1234], cpu.pc), (0x40, 0x0203));
        assert_flags(&cpu, false, false, false, true, "LSR $1234");
    }

    #[test]
    fn dec_memory() {
        let (mut cpu, mut bus) = setup(&[0xC6, 0x90, 0xDE, 0x00, 0x03]);
        cpu.p = StatusFlag::Carry as u8;
        cpu.x = 0x05;
        bus.memory[0x0090] = 0x01;
        bus.memory[0x0305] = 0x00;
        cpu.step(&mut bus);
        assert_eq!(bus.memory[0x0090], 0x00);
        assert_flags(&cpu, false, false, true, true, "DEC $01");
        cpu.step(&mut bus);
        assert_eq!((bus.memory[0x0305], cpu.pc), (0xFF, 0x0205));
        assert_flags(&cpu, true, false, false, true, "DEC $0300,X wraps");
    }

    #[test]
    fn compares_in_memory_modes() {
        let (mut cpu, mut bus) =
            setup(&[0xE4, 0x90, 0xEC, 0x34, 0x12, 0xC4, 0x90, 0xCC, 0x34, 0x12]);
        cpu.x = 0x10;
        cpu.y = 0x05;
        bus.memory[0x0090] = 0x10;
        bus.memory[0x1234] = 0x20;
        cpu.step(&mut bus);
        assert_flags(&cpu, false, false, true, true, "CPX zp equal");
        cpu.step(&mut bus);
        assert_flags(&cpu, true, false, false, false, "CPX abs less");
        cpu.step(&mut bus);
        assert_flags(&cpu, true, false, false, false, "CPY zp less");
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x020A);
    }
}

/// Builds 22 and 24: flag instructions, one each way.
mod status_flag_changes {
    use super::setup;
    use crate::StatusFlag;

    /// Run one flag instruction from `before` and assert the whole of P after.
    fn check(opcode: u8, before: u8, after: u8) {
        let (mut cpu, mut bus) = setup(&[opcode]);
        cpu.p = before;
        cpu.step(&mut bus);
        assert_eq!(cpu.p, after, "opcode ${opcode:02X}");
        assert_eq!(cpu.pc, 0x0201);
    }

    #[test]
    fn test_cli() {
        check(0x58, 0xFF, !(StatusFlag::InterruptDisable as u8));
    }

    #[test]
    fn test_sei() {
        check(0x78, 0x00, StatusFlag::InterruptDisable as u8);
    }

    #[test]
    fn test_cld() {
        check(0xD8, 0xFF, !(StatusFlag::DecimalMode as u8));
    }

    #[test]
    fn test_sed() {
        check(0xF8, 0x00, StatusFlag::DecimalMode as u8);
    }

    #[test]
    fn test_clv() {
        check(0xB8, 0xFF, !(StatusFlag::Overflow as u8));
    }
}

/// Builds 22 and 24: the system instructions.
mod system_functions {
    use super::setup;

    #[test]
    fn test_rti() {
        let (mut cpu, mut bus) = setup(&[0x40]);
        cpu.sp = 0xFC;
        bus.memory[0x01FD] = 0xDF; // pushed P: every bit but U, B set
        bus.memory[0x01FE] = 0x34; // PCL
        bus.memory[0x01FF] = 0x12; // PCH
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x1234, "no +1, unlike RTS");
        assert_eq!(cpu.p, 0xEF, "bit 5 forced on, bit 4 off");
        assert_eq!(cpu.sp, 0xFF);
    }

    #[test]
    fn test_brk_entry_sequence() {
        let (mut cpu, mut bus) = setup(&[]);
        bus.memory[0x0234] = 0x00; // BRK at $0234, so the two pushed PC bytes differ
        bus.memory[0xFFFE..=0xFFFF].copy_from_slice(&[0x00, 0x03]);
        cpu.pc = 0x0234;
        cpu.sp = 0xFF;
        cpu.p = crate::StatusFlag::Carry as u8; // planted, must ride along in the push
        cpu.step(&mut bus);
        assert_eq!(bus.memory[0x01FF], 0x02, "PCH of BRK + 2");
        assert_eq!(bus.memory[0x01FE], 0x36, "PCL of BRK + 2");
        assert_eq!(bus.memory[0x01FD], 0x31, "pushed P: C, plus B and U");
        assert_eq!(cpu.sp, 0xFC);
        assert_eq!(cpu.p, 0x05, "I set afterwards; B not in the live P");
        assert_eq!(cpu.pc, 0x0300, "PC from $FFFE/$FFFF");
    }
}

/// Build 28: the failure model. The kept panics are promises, so each has
/// a test that breaks its invariant on purpose; the no-panic rule has a
/// test that runs a wild program.
mod failure_model {
    use crate::addressing::AddressingMode;
    use crate::opcodes::OPCODES;
    use crate::{Chip6502, FlatBus};

    #[test]
    #[should_panic(expected = "OPCODES table bug")]
    fn resolve_addr_panics_on_a_mode_it_cannot_resolve() {
        // A doctored row: hand a generic handler's resolver Implied.
        let mut bus = FlatBus::new();
        let mut cpu = Chip6502::new();
        let _ = cpu.resolve_addr(AddressingMode::Implied, &mut bus);
    }

    #[test]
    #[should_panic(expected = "routed to not_implemented")]
    fn test_builds_panic_on_a_gap_in_the_table() {
        let mut bus = FlatBus::new();
        let mut cpu = Chip6502::new();
        bus.memory[0x0200] = 0x02; // a dash in Appendix E
        cpu.pc = 0x0200;
        cpu.step(&mut bus);
    }

    #[test]
    fn a_wild_program_never_panics() {
        // Fill memory with documented opcodes, picked by Build 9's LFSR from
        // a fixed seed; operand bytes stay arbitrary, so execution still
        // wanders: wild jumps, stack wrap, self-modifying stores. Peek
        // before each step and stop if a store has planted an undocumented
        // opcode, which would reach the test-build panic above.
        let documented: Vec<u8> = (0..=255u8)
            .filter(|&op| OPCODES[usize::from(op)].mnemonic != "???")
            .collect();
        let mut seed: u8 = 0x01;
        let mut next = || {
            let carry = seed & 0x80 != 0;
            seed <<= 1;
            if carry {
                seed ^= 0x1D;
            }
            seed
        };

        let mut bus = FlatBus::new();
        for byte in &mut bus.memory {
            *byte = documented[usize::from(next()) % documented.len()];
        }
        let mut cpu = Chip6502::new();
        cpu.pc = 0x0200;

        let mut steps = 0;
        for _ in 0..5_000 {
            if OPCODES[usize::from(bus.memory[usize::from(cpu.pc)])].mnemonic == "???" {
                break;
            }
            cpu.step(&mut bus);
            steps += 1;
        }
        // A fill that stopped at once would pass vacuously; make it run.
        assert!(steps >= 300, "the wild program stopped after {steps} steps");
    }
}

/// Build 27: what `step` returns, and the table it reads.
mod cycle_counting {
    use super::setup;
    use crate::addressing::AddressingMode;
    use crate::opcodes::OPCODES;
    use crate::{Chip6502, FlatBus};

    /// Run one instruction placed at `$0200` and return what `step` reported.
    fn cycles_for(program: &[u8], prepare: impl FnOnce(&mut Chip6502, &mut FlatBus)) -> u8 {
        let (mut cpu, mut bus) = setup(program);
        prepare(&mut cpu, &mut bus);
        let before = cpu.cycles;
        let returned = cpu.step(&mut bus);
        assert_eq!(
            cpu.cycles - before,
            u64::from(returned),
            "accumulator matches return"
        );
        returned
    }

    #[test]
    fn baselines() {
        assert_eq!(cycles_for(&[0xA9, 0x01], |_, _| {}), 2, "LDA #imm");
        assert_eq!(cycles_for(&[0xA5, 0x42], |_, _| {}), 3, "LDA zp");
        assert_eq!(cycles_for(&[0xAD, 0x34, 0x12], |_, _| {}), 4, "LDA abs");
    }

    #[test]
    fn indexed_reads_pay_only_on_a_crossing() {
        let x1 = |cpu: &mut Chip6502, _: &mut FlatBus| cpu.x = 0x01;
        assert_eq!(
            cycles_for(&[0xBD, 0xFE, 0x12], x1),
            4,
            "LDA $12FE,X no cross"
        );
        assert_eq!(
            cycles_for(&[0xBD, 0xFF, 0x12], x1),
            5,
            "LDA $12FF,X crosses"
        );
        let y_cross = |cpu: &mut Chip6502, bus: &mut FlatBus| {
            cpu.y = 0x10;
            bus.memory[0x0040..0x0042].copy_from_slice(&[0xF8, 0x12]);
        };
        assert_eq!(cycles_for(&[0xB1, 0x40], y_cross), 6, "LDA ($40),Y crosses");
    }

    #[test]
    fn writes_and_read_modify_writes_never_pay() {
        let x1 = |cpu: &mut Chip6502, _: &mut FlatBus| cpu.x = 0x01;
        assert_eq!(cycles_for(&[0x9D, 0xFF, 0x12], x1), 5, "STA $12FF,X");
        assert_eq!(cycles_for(&[0xFE, 0xFF, 0x12], x1), 7, "INC $12FF,X");
    }

    #[test]
    fn branches_cost_two_three_or_four() {
        assert_eq!(
            cycles_for(&[0xD0, 0x10], |cpu, _| cpu.p = 0x02),
            2,
            "BNE not taken"
        );
        assert_eq!(
            cycles_for(&[0xD0, 0x10], |_, _| {}),
            3,
            "BNE taken, same page"
        );
        assert_eq!(
            cycles_for(&[0xD0, 0x80], |_, _| {}),
            4,
            "BNE taken back into page $01"
        );
        // The page test is measured from the instruction after the branch.
        let (mut cpu, mut bus) = setup(&[]);
        bus.memory[0x02FE..0x0300].copy_from_slice(&[0xD0, 0x00]); // next = $0300, target $0300
        cpu.pc = 0x02FE;
        assert_eq!(
            cpu.step(&mut bus),
            3,
            "same page as the next instruction, though not as the branch"
        );
        bus.memory[0x02FD..0x02FF].copy_from_slice(&[0xD0, 0x01]); // next = $02FF, target $0300
        cpu.pc = 0x02FD;
        assert_eq!(
            cpu.step(&mut bus),
            4,
            "next instruction on page $02, target on $03"
        );
    }

    #[test]
    fn brk_rides_the_table_and_irq_charges_itself() {
        assert_eq!(cycles_for(&[0x00], |_, _| {}), 7, "BRK, not 14");
        let (mut cpu, mut bus) = setup(&[0xEA]);
        cpu.request_irq();
        assert_eq!(cpu.step(&mut bus), 7, "IRQ entry returned");
        assert_eq!(cpu.cycles, 7, "and accumulated");
    }

    #[test]
    fn jmp_indirect_page_bug_still_costs_five() {
        assert_eq!(cycles_for(&[0x6C, 0xFF, 0x12], |_, _| {}), 5);
    }

    #[test]
    fn page_sensitive_rows_are_the_23_documented_reads() {
        let count = OPCODES.iter().filter(|row| row.page_sensitive).count();
        assert_eq!(count, 23);
    }

    /// The base cost Appendix C.1 gives each row, derived from its mnemonic
    /// and mode. C.4's per-mode table covers the plain reads; stores, RMW,
    /// stack and jump instructions follow their own C.1 entries.
    #[expect(
        clippy::match_same_arms,
        reason = "one arm per Appendix C rule; merging equal costs would hide which rule a row follows"
    )]
    fn expected_base(mnemonic: &str, mode: AddressingMode) -> u8 {
        use AddressingMode::{
            Absolute, AbsoluteX, AbsoluteY, Accumulator, Immediate, Implied, IndexedIndirect,
            Indirect, IndirectIndexed, Relative, ZeroPage, ZeroPageX, ZeroPageY,
        };
        let store = matches!(mnemonic, "STA" | "STX" | "STY");
        let rmw = matches!(mnemonic, "ASL" | "LSR" | "ROL" | "ROR" | "INC" | "DEC");
        match (mnemonic, mode) {
            ("PHA" | "PHP", _) => 3,
            ("PLA" | "PLP", _) => 4,
            ("JMP", Absolute) => 3,
            ("JMP", Indirect) => 5,
            ("JSR" | "RTS" | "RTI", _) => 6,
            ("BRK", _) => 7,
            (_, Implied | Accumulator | Immediate | Relative) => 2,
            (_, ZeroPage) if rmw => 5,
            (_, ZeroPage) => 3,
            (_, ZeroPageX) if rmw => 6,
            (_, ZeroPageX | ZeroPageY | Absolute) if !rmw => 4,
            (_, Absolute) => 6,
            (_, AbsoluteX) if rmw => 7,
            (_, AbsoluteX | AbsoluteY) if store => 5,
            (_, AbsoluteX | AbsoluteY) => 4,
            (_, IndexedIndirect) => 6,
            (_, IndirectIndexed) if store => 6,
            (_, IndirectIndexed) => 5,
            (m, mode) => panic!("no C.1 cost for {m} {mode:?}"),
        }
    }

    #[test]
    fn every_row_matches_appendix_c() {
        for (op, row) in OPCODES.iter().enumerate() {
            if row.mnemonic == "???" {
                continue;
            }
            let mode = row.addressing_mode;
            assert_eq!(
                row.cycles,
                expected_base(row.mnemonic, mode),
                "${op:02X} {} {mode:?}",
                row.mnemonic
            );
            let read_indexed = matches!(
                mode,
                AddressingMode::AbsoluteX
                    | AddressingMode::AbsoluteY
                    | AddressingMode::IndirectIndexed
            ) && !matches!(
                row.mnemonic,
                "STA" | "STX" | "STY" | "ASL" | "LSR" | "ROL" | "ROR" | "INC" | "DEC"
            );
            assert_eq!(row.page_sensitive, read_indexed, "${op:02X} page_sensitive");
        }
    }
}

/// Build 25: decimal-mode ADC and SBC, each case run with `SED` first.
mod decimal_mode {
    use super::{assert_flags, setup};
    use crate::StatusFlag;

    /// `SED`, then `LDA #a`, then the carry instruction, then `op #operand`.
    fn run(a: u8, op: u8, operand: u8, carry: bool) -> crate::Chip6502 {
        let carry_op = if carry { 0x38 } else { 0x18 }; // SEC or CLC
        let (mut cpu, mut bus) = setup(&[0xF8, 0xA9, a, carry_op, op, operand]);
        for _ in 0..4 {
            cpu.step(&mut bus);
        }
        assert!(cpu.flag(StatusFlag::DecimalMode));
        cpu
    }

    #[test]
    fn adc_takes_the_decimal_branch() {
        let cpu = run(0x25, 0x69, 0x37, false);
        assert_eq!(cpu.a, 0x62, "not binary $5C");
    }

    #[test]
    fn adc_low_nibble_adjust() {
        let cpu = run(0x27, 0x69, 0x16, false);
        assert_eq!(cpu.a, 0x43);
    }

    #[test]
    fn adc_high_nibble_adjust_and_carry_out() {
        let cpu = run(0x99, 0x69, 0x01, false);
        assert_eq!(cpu.a, 0x00);
        assert!(cpu.flag(StatusFlag::Carry));
    }

    #[test]
    fn sbc_decimal_no_borrow() {
        let cpu = run(0x50, 0xE9, 0x25, true);
        assert_eq!(cpu.a, 0x25);
        assert!(cpu.flag(StatusFlag::Carry));
    }

    #[test]
    fn adc_nmos_flag_quirk() {
        // The BCD result is $00, but Z comes from the binary sum $9A and N
        // from the pre-adjust intermediate $A0: a "fixed" core gives N=0, Z=1.
        let cpu = run(0x99, 0x69, 0x01, false);
        assert_eq!(cpu.a, 0x00);
        assert_flags(&cpu, true, false, false, true, "$99 + $01 decimal");
    }

    #[test]
    fn sbc_borrow_out_gives_tens_complement() {
        let cpu = run(0x25, 0xE9, 0x50, true);
        assert_eq!(cpu.a, 0x75, "−25 as ten's complement");
        assert!(!cpu.flag(StatusFlag::Carry), "borrow out");
    }

    #[test]
    fn sbc_with_incoming_borrow() {
        let cpu = run(0x30, 0xE9, 0x25, false);
        assert_eq!(cpu.a, 0x04, "$30 - $25 - 1");
    }
}

/// Build 23 §12: interrupt behavior, one rule per test.
mod interrupts {
    use crate::{Chip6502, FlatBus, StatusFlag};

    /// A bus with the three vectors written (NMI → `$0400`, RESET → `$0200`,
    /// IRQ/BRK → `$0300`), `NOP`s at all three targets, and a CPU reset onto it.
    fn fixture() -> (Chip6502, FlatBus) {
        let mut bus = FlatBus::new();
        bus.memory[0xFFFA..=0xFFFF].copy_from_slice(&[0x00, 0x04, 0x00, 0x02, 0x00, 0x03]);
        for addr in [0x0200, 0x0300, 0x0400] {
            bus.memory[addr..addr + 16].fill(0xEA); // NOP
        }
        let mut cpu = Chip6502::new();
        cpu.sp = 0xFF;
        cpu.reset(&mut bus);
        (cpu, bus)
    }

    #[test]
    fn reset_loads_the_vector() {
        let mut bus = FlatBus::new();
        bus.memory[0xFFFC..=0xFFFD].copy_from_slice(&[0x34, 0x12]);
        let mut cpu = Chip6502::new();
        cpu.reset(&mut bus);
        assert_eq!(cpu.pc, 0x1234);
    }

    #[test]
    fn reset_drops_sp_by_three_and_sets_i_without_writing() {
        let mut bus = FlatBus::new();
        let mut cpu = Chip6502::new();
        cpu.sp = 0x40;
        cpu.reset(&mut bus);
        assert_eq!(cpu.sp, 0x3D);
        assert!(cpu.flag(StatusFlag::InterruptDisable));
        assert!(
            bus.memory[0x0100..=0x01FF].iter().all(|&b| b == 0),
            "no stack writes"
        );
    }

    #[test]
    fn irq_does_nothing_when_masked() {
        let (mut cpu, mut bus) = fixture(); // reset left I = 1
        cpu.request_irq();
        for _ in 0..3 {
            cpu.step(&mut bus);
        }
        assert_eq!(cpu.pc, 0x0203, "still running the main program");
    }

    #[test]
    fn irq_fires_when_unmasked_and_pushes_b_clear() {
        let (mut cpu, mut bus) = fixture();
        cpu.set_flag(StatusFlag::InterruptDisable, false);
        cpu.step(&mut bus); // one NOP: PC = $0201
        cpu.request_irq();
        let sp_before = cpu.sp;
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x0300);
        assert_eq!(cpu.sp, sp_before.wrapping_sub(3));
        let pushed_p = bus.memory[0x0100 + usize::from(cpu.sp) + 1];
        assert_eq!(
            pushed_p & StatusFlag::BreakCommand as u8,
            0,
            "B clear for IRQ"
        );
        assert_ne!(pushed_p & StatusFlag::Unused as u8, 0, "U set");
        assert_eq!(
            bus.memory[0x0100 + usize::from(cpu.sp) + 2],
            0x01,
            "PCL of $0201"
        );
    }

    #[test]
    fn nmi_fires_regardless_of_i_and_clears_itself() {
        let (mut cpu, mut bus) = fixture(); // I = 1
        cpu.trigger_nmi();
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x0400);
        assert!(!cpu.nmi_pending);
    }

    #[test]
    fn nmi_is_edge_triggered() {
        let (mut cpu, mut bus) = fixture();
        cpu.trigger_nmi();
        cpu.step(&mut bus);
        for _ in 0..8 {
            cpu.step(&mut bus);
        }
        assert_eq!(
            cpu.pc, 0x0408,
            "eight NOPs into the handler, no second entry"
        );
    }

    #[test]
    fn irq_is_level_triggered() {
        let (mut cpu, mut bus) = fixture();
        cpu.set_flag(StatusFlag::InterruptDisable, false);
        cpu.request_irq();
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x0300);
        cpu.set_flag(StatusFlag::InterruptDisable, false); // as RTI would restore it
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x0300, "the held line fires again");
        cpu.clear_irq();
        cpu.set_flag(StatusFlag::InterruptDisable, false);
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x0301, "released: the handler runs");
    }

    #[test]
    fn brk_pushes_b_set_and_returns_two_bytes_on() {
        let (mut cpu, mut bus) = fixture();
        bus.memory[0x0200..0x0202].copy_from_slice(&[0x00, 0x00]); // BRK + marker
        bus.memory[0x0300] = 0x40; // handler: RTI
        cpu.step(&mut bus);
        let pushed_p = bus.memory[0x0100 + usize::from(cpu.sp) + 1];
        assert_ne!(
            pushed_p & StatusFlag::BreakCommand as u8,
            0,
            "B set for BRK"
        );
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x0202, "BRK address + 2, not + 1");
    }

    #[test]
    fn rti_restores_pc_and_p_after_an_interrupt() {
        let (mut cpu, mut bus) = fixture();
        bus.memory[0x0300] = 0x40; // handler: RTI
        cpu.p = StatusFlag::Carry as u8 | StatusFlag::Unused as u8; // I clear
        cpu.step(&mut bus); // NOP at $0200
        cpu.request_irq();
        cpu.step(&mut bus); // enter
        cpu.clear_irq();
        cpu.step(&mut bus); // RTI
        assert_eq!(cpu.pc, 0x0201);
        assert_eq!(cpu.p, StatusFlag::Carry as u8 | StatusFlag::Unused as u8);
    }

    #[test]
    fn brk_and_irq_share_the_fffe_vector() {
        let (mut cpu, mut bus) = fixture();
        bus.memory[0x0200] = 0x00;
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x0300, "BRK");
        let (mut cpu, mut bus) = fixture();
        cpu.set_flag(StatusFlag::InterruptDisable, false);
        cpu.request_irq();
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x0300, "IRQ");
    }

    #[test]
    fn entry_leaves_d_alone() {
        let (mut cpu, mut bus) = fixture();
        cpu.set_flag(StatusFlag::DecimalMode, true);
        cpu.trigger_nmi();
        cpu.step(&mut bus);
        assert!(
            cpu.flag(StatusFlag::DecimalMode),
            "NMOS entry does not clear D"
        );
    }
}

/// Build 22's architecture gate. The suite proves the refactor broke
/// nothing; these checks prove it was done: one dispatcher, no surviving
/// per-mode methods, no second opcode map, and table and dispatch agreeing
/// about which opcodes exist.
mod architecture_gate {
    use crate::opcodes::OPCODES;
    use crate::{Chip6502, FlatBus};
    use std::collections::BTreeSet;

    /// The crate's non-test source files, read at test time so that this
    /// file's own text is never searched.
    fn library_sources() -> Vec<(String, String)> {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        let mut sources = Vec::new();
        for entry in std::fs::read_dir(dir).expect("read src/") {
            let path = entry.expect("directory entry").path();
            let name = path
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .into_owned();
            if path.extension().is_some_and(|ext| ext == "rs") && !name.starts_with("tests_") {
                sources.push((name, std::fs::read_to_string(&path).expect("read source")));
            }
        }
        sources
    }

    /// The `match opcode { … }` body of `dispatch`.
    fn dispatch_body() -> String {
        let source =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/opcodes.rs"))
                .expect("read opcodes.rs");
        let start = source
            .find(concat!("match", " opcode {"))
            .expect("dispatch match");
        let end = start + source[start..].find("_ =>").expect("catch-all arm");
        source[start..end].to_string()
    }

    #[test]
    fn every_table_row_reaches_a_real_handler() {
        for op in 0..=255u8 {
            if OPCODES[usize::from(op)].mnemonic == "???" {
                continue;
            }
            let mut bus = FlatBus::new();
            let mut cpu = Chip6502::new();
            bus.memory[0x0200] = op;
            cpu.pc = 0x0200;
            cpu.step(&mut bus); // not_implemented panics under cfg(test)
        }
    }

    #[test]
    fn the_table_has_151_rows() {
        let mut rows = 0;
        for info in &OPCODES {
            if info.mnemonic != "???" {
                rows += 1;
            }
        }
        assert_eq!(rows, 151, "the documented NMOS set");
    }

    // Beyond the book: the reverse check (an arm with no row), read from
    // the source text of `dispatch`.

    #[test]
    fn table_and_dispatch_name_the_same_opcodes() {
        let body = dispatch_body();
        let dispatched: BTreeSet<u8> = body
            .match_indices("0x")
            .map(|(i, _)| u8::from_str_radix(&body[i + 2..i + 4], 16).expect("two hex digits"))
            .collect();
        let tabled: BTreeSet<u8> = (0..=255u8)
            .filter(|&op| OPCODES[usize::from(op)].mnemonic != "???")
            .collect();
        assert_eq!(dispatched, tabled);
        assert_eq!(tabled.len(), 151, "the documented NMOS set");
    }

    #[test]
    fn there_is_one_dispatch_site() {
        let needle = concat!("match", " opcode");
        let count: usize = library_sources()
            .iter()
            .map(|(_, text)| text.matches(needle).count())
            .sum();
        assert_eq!(count, 1, "`{needle}` must appear exactly once");
    }

    #[test]
    fn the_old_opcode_map_is_gone() {
        let needle = concat!("opcode", "_info");
        for (name, text) in library_sources() {
            assert!(!text.contains(needle), "{name} still mentions `{needle}`");
        }
    }

    #[test]
    fn no_per_mode_methods_survive_for_generic_instructions() {
        const GENERIC: [&str; 21] = [
            "lda", "ldx", "ldy", "sta", "stx", "sty", "adc", "sbc", "cmp", "cpx", "cpy", "and",
            "ora", "eor", "bit", "asl", "lsr", "rol", "ror", "inc", "dec",
        ];
        const MODES: [&str; 6] = [
            "immediate",
            "zero_page",
            "absolute",
            "indexed_indirect",
            "indirect_indexed",
            "relative",
        ];
        for (name, text) in library_sources() {
            for mnemonic in GENERIC {
                for mode in MODES {
                    let method = format!("fn {mnemonic}_{mode}");
                    assert!(!text.contains(&method), "{name} still defines `{method}`");
                }
            }
        }
    }
}

/// Build 3: the disassembler

#[test]
fn cpx_immediate() {
    let mut bus = FlatBus::new();
    let mut cpu = Chip6502::new();
    cpu.p = StatusFlag::Overflow as u8; // V is the decoy: compares never move it
    bus.memory[0x0200..0x0206].copy_from_slice(&[0xE0, 0x10, 0xE0, 0x08, 0xE0, 0x20]);
    cpu.pc = 0x0200;
    cpu.x = 0x10;

    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0202);
    assert_flags(&cpu, false, true, true, true, "CPX equal");

    cpu.step(&mut bus);
    assert_flags(&cpu, false, true, false, true, "CPX greater");

    cpu.step(&mut bus);
    assert_flags(&cpu, true, true, false, false, "CPX less");
    assert_eq!(cpu.x, 0x10, "CPX leaves X alone");
}

#[test]
fn jsr_rts_round_trip() {
    let mut bus = FlatBus::new();
    let mut cpu = Chip6502::new();
    cpu.sp = 0xFF;
    cpu.pc = 0x0234;
    bus.memory[0x0234..0x0237].copy_from_slice(&[0x20, 0x00, 0x03]); // JSR $0300
    bus.memory[0x0300] = 0x60; // RTS

    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0300, "JSR lands on its target");
    assert_eq!(cpu.sp, 0xFD, "JSR pushed two bytes");
    assert_eq!(bus.memory[0x01FF], 0x02, "high byte of PC + 2 pushed first");
    assert_eq!(bus.memory[0x01FE], 0x36, "low byte of PC + 2 pushed second");

    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0237, "RTS adds one: the byte after the JSR");
    assert_eq!(cpu.sp, 0xFF, "SP back where it started");
}

/// A fresh CPU with `program` loaded at `$0200` and PC pointing at it.
fn setup(program: &[u8]) -> (Chip6502, FlatBus) {
    let mut bus = FlatBus::new();
    let mut cpu = Chip6502::new();
    bus.memory[0x0200..0x0200 + program.len()].copy_from_slice(program);
    cpu.pc = 0x0200;
    (cpu, bus)
}

// ----- Build 4: print via zero-page indirect pointer -----

#[test]
fn sta_zero_page_stores_a() {
    let (mut cpu, mut bus) = setup(&[0x85, 0x80]); // STA $80
    cpu.a = 0x5A;
    cpu.step(&mut bus);
    assert_eq!(bus.memory[0x0080], 0x5A);
    assert_eq!(cpu.pc, 0x0202);
}

#[test]
fn ldy_immediate_loads_y_and_sets_flags() {
    let (mut cpu, mut bus) = setup(&[0xA0, 0x44, 0xA0, 0x00, 0xA0, 0x80]);
    cpu.p |= StatusFlag::Carry as u8;
    cpu.step(&mut bus);
    assert_eq!((cpu.y, cpu.pc), (0x44, 0x0202));
    assert_flags(&cpu, false, false, false, true, "LDY #$44");
    cpu.step(&mut bus);
    assert_eq!(cpu.y, 0x00);
    assert_flags(&cpu, false, false, true, true, "LDY #$00");
    cpu.step(&mut bus);
    assert_eq!(cpu.y, 0x80);
    assert_flags(&cpu, true, false, false, true, "LDY #$80");
}

#[test]
fn iny_increments_y_and_sets_flags() {
    let (mut cpu, mut bus) = setup(&[0xC8, 0xC8, 0xC8]);
    cpu.p |= StatusFlag::Carry as u8;
    cpu.y = 0x10;
    cpu.step(&mut bus);
    assert_eq!((cpu.y, cpu.pc), (0x11, 0x0201));
    assert_flags(&cpu, false, false, false, true, "INY $10");
    cpu.y = 0xFF;
    cpu.step(&mut bus);
    assert_eq!(cpu.y, 0x00);
    assert_flags(&cpu, false, false, true, true, "INY $FF wraps");
    cpu.y = 0x7F;
    cpu.step(&mut bus);
    assert_eq!(cpu.y, 0x80);
    assert_flags(&cpu, true, false, false, true, "INY $7F");
}

#[test]
fn lda_indirect_indexed() {
    // Ordinary case: pointer at $80/$81 = $0300, Y = 5; decoy at $0300.
    let (mut cpu, mut bus) = setup(&[0xB1, 0x80, 0xB1, 0xFF]);
    cpu.p |= StatusFlag::Carry as u8;
    bus.memory[0x0080] = 0x00;
    bus.memory[0x0081] = 0x03;
    bus.memory[0x0300] = 0xEE; // the answer if Y is ignored
    bus.memory[0x0305] = 0x80; // the answer if Y is added
    cpu.y = 0x05;
    cpu.step(&mut bus);
    assert_eq!((cpu.a, cpu.pc), (0x80, 0x0202));
    assert_flags(&cpu, true, false, false, true, "LDA ($80),Y");

    // The trap: operand $FF takes its high byte from $00, never $0100.
    bus.memory[0x00FF] = 0x00;
    bus.memory[0x0000] = 0x04;
    bus.memory[0x0100] = 0x05;
    bus.memory[0x0405] = 0x00; // read through $0400 + Y
    bus.memory[0x0505] = 0x77; // read if the fetch escaped page zero
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00, "pointer high byte must wrap to $0000");
    assert_flags(&cpu, false, false, true, true, "LDA ($FF),Y");
}

// ----- Build 5: hex byte output -----

#[test]
fn pha_pushes_a() {
    let (mut cpu, mut bus) = setup(&[0x48]);
    cpu.sp = 0xFF;
    cpu.a = 0x42;
    cpu.p = StatusFlag::Zero as u8; // PHA moves no flags
    cpu.step(&mut bus);
    assert_eq!(bus.memory[0x01FF], 0x42);
    assert_eq!((cpu.sp, cpu.pc), (0xFE, 0x0201));
    assert_flags(&cpu, false, false, true, false, "PHA");
}

#[test]
fn pla_pops_into_a_and_sets_flags() {
    let (mut cpu, mut bus) = setup(&[0x68, 0x68, 0x68]);
    cpu.sp = 0xFC;
    cpu.p |= StatusFlag::Carry as u8;
    bus.memory[0x01FD] = 0x44;
    bus.memory[0x01FE] = 0x00;
    bus.memory[0x01FF] = 0x80;
    cpu.step(&mut bus);
    assert_eq!((cpu.a, cpu.sp, cpu.pc), (0x44, 0xFD, 0x0201));
    assert_flags(&cpu, false, false, false, true, "PLA $44");
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00);
    assert_flags(&cpu, false, false, true, true, "PLA $00");
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x80);
    assert_flags(&cpu, true, false, false, true, "PLA $80");
}

#[test]
fn pha_pla_round_trip() {
    let (mut cpu, mut bus) = setup(&[0x48, 0xA9, 0x00, 0x68]); // PHA; LDA #0; PLA
    cpu.sp = 0xFF;
    cpu.a = 0x42;
    for _ in 0..3 {
        cpu.step(&mut bus);
    }
    assert_eq!((cpu.a, cpu.sp), (0x42, 0xFF));
}

#[test]
fn lsr_accumulator_shifts_into_carry() {
    let (mut cpu, mut bus) = setup(&[0x4A, 0x4A]);
    cpu.p = StatusFlag::Negative as u8; // N must end clear
    cpu.a = 0b0000_0011;
    cpu.step(&mut bus);
    assert_eq!((cpu.a, cpu.pc), (0b0000_0001, 0x0201));
    assert_flags(&cpu, false, false, false, true, "LSR %00000011");
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00);
    assert_flags(&cpu, false, false, true, true, "LSR %00000001");
}

#[test]
fn and_immediate_masks_a() {
    let (mut cpu, mut bus) = setup(&[0x29, 0x0F, 0x29, 0xF0, 0x29, 0x80]);
    cpu.p |= StatusFlag::Carry as u8;
    cpu.a = 0xAB;
    cpu.step(&mut bus);
    assert_eq!((cpu.a, cpu.pc), (0x0B, 0x0202));
    assert_flags(&cpu, false, false, false, true, "$AB AND $0F");
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00);
    assert_flags(&cpu, false, false, true, true, "$0B AND $F0");
    cpu.a = 0xC3;
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x80);
    assert_flags(&cpu, true, false, false, true, "$C3 AND $80");
}

#[test]
fn cmp_immediate() {
    let (mut cpu, mut bus) = setup(&[0xC9, 0x10, 0xC9, 0x08, 0xC9, 0x20]);
    cpu.p = StatusFlag::Overflow as u8;
    cpu.a = 0x10;
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0202);
    assert_flags(&cpu, false, true, true, true, "CMP equal");
    cpu.step(&mut bus);
    assert_flags(&cpu, false, true, false, true, "CMP greater");
    cpu.step(&mut bus);
    assert_flags(&cpu, true, true, false, false, "CMP less");
    assert_eq!(cpu.a, 0x10, "CMP leaves A alone");
}

#[test]
fn bcc_taken_and_not_taken() {
    let (mut cpu, mut bus) = setup(&[0x90, 0x06]);
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0208, "C clear: taken, $0200 + 2 + 6");

    cpu.pc = 0x0200;
    cpu.p = StatusFlag::Carry as u8;
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0202, "C set: not taken");

    bus.memory[0x0210..0x0212].copy_from_slice(&[0x90, 0xF0]);
    cpu.pc = 0x0210;
    cpu.p = 0;
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0202, "backwards: $0210 + 2 - 16");
}

// ----- Build 6: memory dump -----

#[test]
fn lda_zero_page_loads_a_and_sets_flags() {
    let (mut cpu, mut bus) = setup(&[0xA5, 0x80, 0xA5, 0x81, 0xA5, 0x82]);
    cpu.p |= StatusFlag::Carry as u8;
    bus.memory[0x0080..0x0083].copy_from_slice(&[0x44, 0x00, 0x80]);
    cpu.step(&mut bus);
    assert_eq!((cpu.a, cpu.pc), (0x44, 0x0202));
    assert_flags(&cpu, false, false, false, true, "LDA $80");
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00);
    assert_flags(&cpu, false, false, true, true, "LDA $81");
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x80);
    assert_flags(&cpu, true, false, false, true, "LDA $82");
}

#[test]
fn cpy_immediate() {
    let (mut cpu, mut bus) = setup(&[0xC0, 0x10, 0xC0, 0x08, 0xC0, 0x20]);
    cpu.p = StatusFlag::Overflow as u8;
    cpu.y = 0x10;
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0202);
    assert_flags(&cpu, false, true, true, true, "CPY equal");
    cpu.step(&mut bus);
    assert_flags(&cpu, false, true, false, true, "CPY greater");
    cpu.step(&mut bus);
    assert_flags(&cpu, true, true, false, false, "CPY less");
}

#[test]
fn inc_zero_page() {
    let (mut cpu, mut bus) = setup(&[0xE6, 0x80, 0xE6, 0x80, 0xE6, 0x81]);
    cpu.p |= StatusFlag::Carry as u8;
    bus.memory[0x0080] = 0xFE;
    bus.memory[0x0081] = 0x7F;
    cpu.step(&mut bus);
    assert_eq!((bus.memory[0x0080], cpu.pc), (0xFF, 0x0202));
    assert_flags(&cpu, true, false, false, true, "INC $FE");
    cpu.step(&mut bus);
    assert_eq!(bus.memory[0x0080], 0x00);
    assert_flags(&cpu, false, false, true, true, "INC $FF wraps");
    cpu.step(&mut bus);
    assert_eq!(bus.memory[0x0081], 0x80);
    assert_flags(&cpu, true, false, false, true, "INC $7F");
}

// ----- Build 8: print 99 down to 0 in decimal -----

#[test]
fn tax_implied() {
    let (mut cpu, mut bus) = setup(&[0xAA, 0xAA, 0xAA]);
    cpu.p = StatusFlag::Carry as u8 | StatusFlag::Overflow as u8; // decoys
    cpu.a = 0x44;
    cpu.step(&mut bus);
    assert_eq!((cpu.x, cpu.pc), (0x44, 0x0201));
    assert_flags(&cpu, false, true, false, true, "TAX $44");
    cpu.a = 0x00;
    cpu.step(&mut bus);
    assert_eq!(cpu.x, 0x00);
    assert_flags(&cpu, false, true, true, true, "TAX $00");
    cpu.a = 0x80;
    cpu.step(&mut bus);
    assert_eq!(cpu.x, 0x80);
    assert_flags(&cpu, true, true, false, true, "TAX $80");
}

#[test]
fn dex_implied() {
    let (mut cpu, mut bus) = setup(&[0xCA, 0xCA, 0xCA]);
    cpu.p = StatusFlag::Carry as u8;
    cpu.x = 0x42;
    cpu.step(&mut bus);
    assert_eq!((cpu.x, cpu.pc), (0x41, 0x0201));
    assert_flags(&cpu, false, false, false, true, "DEX $42");
    cpu.x = 0x01;
    cpu.step(&mut bus);
    assert_eq!(cpu.x, 0x00);
    assert_flags(&cpu, false, false, true, true, "DEX $01");
    cpu.step(&mut bus);
    assert_eq!(cpu.x, 0xFF);
    assert_flags(&cpu, true, false, false, true, "DEX $00 wraps");
}

#[test]
fn bpl_relative() {
    let (mut cpu, mut bus) = setup(&[0x10, 0x06]);
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0208, "N clear: taken forwards");

    bus.memory[0x020F..0x0211].copy_from_slice(&[0x10, 0xF1]); // the program's offset
    cpu.pc = 0x020F;
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0202, "taken backwards: $020F + 2 - 15");

    cpu.pc = 0x0200;
    cpu.p = StatusFlag::Negative as u8;
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0202, "N set: not taken");
    assert_flags(&cpu, true, false, false, false, "BPL moves no flag");

    bus.memory[0x0300..0x0302].copy_from_slice(&[0x10, 0x00]);
    cpu.pc = 0x0300;
    cpu.p = 0;
    cpu.step(&mut bus);
    assert_eq!(
        cpu.pc, 0x0302,
        "zero offset: taken, lands on the next instruction"
    );
}

// ----- Build 9: pseudo-random number generator (LFSR) -----

#[test]
fn asl_accumulator() {
    let (mut cpu, mut bus) = setup(&[0x0A, 0x0A, 0x0A, 0x0A]);
    cpu.p = StatusFlag::Overflow as u8; // V must not move
    cpu.a = 0xC1;
    cpu.step(&mut bus);
    assert_eq!((cpu.a, cpu.pc), (0x82, 0x0201));
    assert_flags(&cpu, true, true, false, true, "ASL $C1");
    cpu.a = 0x41;
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x82);
    assert_flags(&cpu, true, true, false, false, "ASL $41");
    cpu.a = 0x80;
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00);
    assert_flags(&cpu, false, true, true, true, "ASL $80: Z and C together");
    cpu.a = 0x01;
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x02);
    assert_flags(&cpu, false, true, false, false, "ASL $01");
}

#[test]
fn eor_immediate() {
    let (mut cpu, mut bus) = setup(&[0x49, 0x0F, 0x49, 0xFF, 0x49, 0x55]);
    cpu.p = StatusFlag::Carry as u8 | StatusFlag::Overflow as u8;
    cpu.a = 0x0A;
    cpu.step(&mut bus);
    assert_eq!((cpu.a, cpu.pc), (0x05, 0x0202));
    assert_flags(&cpu, false, true, false, true, "$0A EOR $0F");
    cpu.a = 0xFF;
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00);
    assert_flags(&cpu, false, true, true, true, "$FF EOR $FF");
    cpu.a = 0xAA;
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0xFF);
    assert_flags(&cpu, true, true, false, true, "$AA EOR $55");
}

// ----- Build 10: multiplication via shift-and-add -----

#[test]
fn dey_implied() {
    let (mut cpu, mut bus) = setup(&[0x88, 0x88, 0x88]);
    cpu.p = StatusFlag::Carry as u8;
    cpu.y = 0x01;
    cpu.step(&mut bus);
    assert_eq!((cpu.y, cpu.pc), (0x00, 0x0201));
    assert_flags(&cpu, false, false, true, true, "DEY $01");
    cpu.step(&mut bus);
    assert_eq!(cpu.y, 0xFF);
    assert_flags(&cpu, true, false, false, true, "DEY $00 wraps");
    cpu.y = 0x80;
    cpu.step(&mut bus);
    assert_eq!(cpu.y, 0x7F);
    assert_flags(&cpu, false, false, false, true, "DEY $80");
}

#[test]
fn asl_zero_page() {
    let (mut cpu, mut bus) = setup(&[0x06, 0x90, 0x06, 0x91]);
    cpu.a = 0x33; // A must not change
    bus.memory[0x0090] = 0xC1;
    bus.memory[0x0091] = 0x41;
    cpu.step(&mut bus);
    assert_eq!((bus.memory[0x0090], cpu.a, cpu.pc), (0x82, 0x33, 0x0202));
    assert_flags(&cpu, true, false, false, true, "ASL $90 = $C1");
    cpu.step(&mut bus);
    assert_eq!(bus.memory[0x0091], 0x82);
    assert_flags(&cpu, true, false, false, false, "ASL $91 = $41");
}

// ----- Build 11: jump-table dispatch -----

#[test]
fn jmp_indirect() {
    // Normal case: the pointer is at $0214/$0215.
    let (mut cpu, mut bus) = setup(&[0x6C, 0x14, 0x02]);
    bus.memory[0x0214] = 0x28;
    bus.memory[0x0215] = 0x02;
    cpu.p = StatusFlag::Carry as u8;
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0228);
    assert_flags(&cpu, false, false, false, true, "JMP ($0214) moves no flag");

    // Bug case: $10FF takes its high byte from $1000, not $1100.
    bus.memory[0x0300..0x0303].copy_from_slice(&[0x6C, 0xFF, 0x10]);
    bus.memory[0x10FF] = 0x34;
    bus.memory[0x1000] = 0x12;
    bus.memory[0x1100] = 0x99; // the decoy: target $9934 if the wrap is missing
    cpu.pc = 0x0300;
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x1234);

    // Edge case: the wrap applies in page zero too.
    bus.memory[0x0400..0x0403].copy_from_slice(&[0x6C, 0xFF, 0x00]);
    bus.memory[0x00FF] = 0x78;
    bus.memory[0x0000] = 0x56;
    bus.memory[0x0100] = 0xAA;
    cpu.pc = 0x0400;
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x5678);
}

// ----- Build 12: 16-bit decimal output -----

#[test]
fn sec_sets_carry() {
    let (mut cpu, mut bus) = setup(&[0x38, 0x38]);
    cpu.p = StatusFlag::Zero as u8; // the decoy
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0201);
    assert_flags(&cpu, false, false, true, true, "SEC");
    cpu.step(&mut bus);
    assert_flags(
        &cpu,
        false,
        false,
        true,
        true,
        "SEC again: sets, never toggles",
    );
}

// ----- Build 14: division via shift-and-subtract -----

#[test]
fn rol_accumulator() {
    let (mut cpu, mut bus) = setup(&[0x2A, 0x2A, 0x2A, 0x2A]);
    cpu.p = StatusFlag::Carry as u8 | StatusFlag::Overflow as u8;
    cpu.a = 0b1100_0001;
    cpu.step(&mut bus);
    assert_eq!((cpu.a, cpu.pc), (0b1000_0011, 0x0201));
    assert_flags(&cpu, true, true, false, true, "ROL %11000001, C=1");
    cpu.p = StatusFlag::Overflow as u8;
    cpu.a = 0b1000_0000;
    cpu.step(&mut bus);
    assert_eq!(
        cpu.a, 0x00,
        "the sampling-order trap: must not become %00000001"
    );
    assert_flags(&cpu, false, true, true, true, "ROL %10000000, C=0");
    cpu.a = 0x40; // C is now 1
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x81);
    assert_flags(&cpu, true, true, false, false, "ROL $40, C=1");
    cpu.a = 0x00;
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00);
    assert_flags(&cpu, false, true, true, false, "ROL $00, C=0");
}

#[test]
fn rol_zero_page() {
    let (mut cpu, mut bus) = setup(&[0x26, 0x90, 0x26, 0x90]);
    cpu.p = StatusFlag::Carry as u8;
    cpu.a = 0x33;
    bus.memory[0x0090] = 0b1100_0001;
    cpu.step(&mut bus);
    assert_eq!(
        (bus.memory[0x0090], cpu.a, cpu.pc),
        (0b1000_0011, 0x33, 0x0202)
    );
    assert_flags(&cpu, true, false, false, true, "ROL $90");
    cpu.p = 0;
    bus.memory[0x0090] = 0b1000_0000;
    cpu.step(&mut bus);
    assert_eq!(bus.memory[0x0090], 0x00);
    assert_flags(&cpu, false, false, true, true, "ROL $90 = $80, C=0");
}

#[test]
fn cmp_zero_page() {
    let (mut cpu, mut bus) = setup(&[0xC5, 0x90, 0xC5, 0x91, 0xC5, 0x92]);
    cpu.p = StatusFlag::Overflow as u8;
    cpu.a = 0x10;
    bus.memory[0x0090..0x0093].copy_from_slice(&[0x10, 0x08, 0x20]);
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0202);
    assert_flags(&cpu, false, true, true, true, "CMP zp equal");
    cpu.step(&mut bus);
    assert_flags(&cpu, false, true, false, true, "CMP zp greater");
    cpu.step(&mut bus);
    assert_flags(&cpu, true, true, false, false, "CMP zp less");
    assert_eq!(cpu.a, 0x10);
}

#[test]
fn ldx_zero_page() {
    let (mut cpu, mut bus) = setup(&[0xA6, 0x90, 0xA6, 0x91, 0xA6, 0x92]);
    cpu.p = StatusFlag::Carry as u8;
    bus.memory[0x0090..0x0093].copy_from_slice(&[0x44, 0x00, 0x80]);
    cpu.step(&mut bus);
    assert_eq!((cpu.x, cpu.pc), (0x44, 0x0202));
    assert_flags(&cpu, false, false, false, true, "LDX $90");
    cpu.step(&mut bus);
    assert_eq!(cpu.x, 0x00);
    assert_flags(&cpu, false, false, true, true, "LDX $91");
    cpu.step(&mut bus);
    assert_eq!(cpu.x, 0x80);
    assert_flags(&cpu, true, false, false, true, "LDX $92");
}

#[test]
fn stx_zero_page() {
    let (mut cpu, mut bus) = setup(&[0x86, 0x90]);
    cpu.p = StatusFlag::Negative as u8 | StatusFlag::Zero as u8; // must survive
    cpu.x = 0x07;
    cpu.step(&mut bus);
    assert_eq!((bus.memory[0x0090], cpu.pc), (0x07, 0x0202));
    assert_flags(&cpu, true, false, true, false, "STX moves no flags");
}

// ----- Build 15: bit-field / status-byte manipulation -----

#[test]
fn ora_immediate() {
    let (mut cpu, mut bus) = setup(&[0x09, 0x0F, 0x09, 0x00, 0x09, 0x01]);
    cpu.p = StatusFlag::Carry as u8 | StatusFlag::Overflow as u8;
    cpu.a = 0x3C;
    cpu.step(&mut bus);
    assert_eq!((cpu.a, cpu.pc), (0x3F, 0x0202));
    assert_flags(&cpu, false, true, false, true, "$3C ORA $0F");
    cpu.a = 0x00;
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00);
    assert_flags(&cpu, false, true, true, true, "$00 ORA $00");
    cpu.a = 0x80;
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x81);
    assert_flags(&cpu, true, true, false, true, "$80 ORA $01");
}

#[test]
fn ora_zero_page() {
    let (mut cpu, mut bus) = setup(&[0x05, 0x90]);
    cpu.a = 0x3C;
    bus.memory[0x0090] = 0x81;
    cpu.step(&mut bus);
    assert_eq!((cpu.a, cpu.pc), (0xBD, 0x0202));
    assert_flags(&cpu, true, false, false, false, "ORA $90");
}

#[test]
fn ora_absolute() {
    let (mut cpu, mut bus) = setup(&[0x0D, 0x34, 0x12]);
    cpu.a = 0x01;
    bus.memory[0x1234] = 0x02;
    bus.memory[0x3412] = 0x80; // the answer if the address bytes are swapped
    cpu.step(&mut bus);
    assert_eq!((cpu.a, cpu.pc), (0x03, 0x0203));
    assert_flags(&cpu, false, false, false, false, "ORA $1234");
}

#[test]
fn bit_zero_page() {
    let (mut cpu, mut bus) = setup(&[0x24, 0x90, 0x24, 0x91]);
    cpu.p = StatusFlag::Carry as u8;
    cpu.a = 0x3F;
    bus.memory[0x0090] = 0xC0;
    bus.memory[0x0091] = 0x0F;
    cpu.step(&mut bus);
    assert_eq!((cpu.a, cpu.pc), (0x3F, 0x0202), "BIT leaves A alone");
    assert_flags(
        &cpu,
        true,
        true,
        true,
        true,
        "BIT $C0: N, V from memory; Z from A & M",
    );
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x3F);
    assert_flags(&cpu, false, false, false, true, "BIT $0F");
}

#[test]
fn bit_absolute() {
    let (mut cpu, mut bus) = setup(&[0x2C, 0x4D, 0xFE]);
    cpu.a = 0x3F;
    bus.memory[0xFE4D] = 0xC0;
    cpu.step(&mut bus);
    assert_eq!((cpu.a, cpu.pc), (0x3F, 0x0203));
    assert_flags(&cpu, true, true, true, false, "BIT $FE4D");
}

#[test]
fn bvc_relative() {
    let (mut cpu, mut bus) = setup(&[0x50, 0x05]);
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0207, "V clear: taken");
    cpu.pc = 0x0200;
    cpu.p = StatusFlag::Overflow as u8;
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0202, "V set: not taken");
}

// ----- Build 17: memory copy -----

#[test]
fn sta_indirect_indexed_stores_through_pointer() {
    let (mut cpu, mut bus) = setup(&[0x91, 0x80]);
    cpu.p = StatusFlag::Zero as u8 | StatusFlag::Carry as u8; // flags as planted
    bus.memory[0x0080] = 0x00;
    bus.memory[0x0081] = 0x04;
    bus.memory[0x0400] = 0x11; // decoy at the unindexed base
    cpu.y = 0x05;
    cpu.a = 0xAB;
    cpu.step(&mut bus);
    assert_eq!(bus.memory[0x0405], 0xAB);
    assert_eq!(bus.memory[0x0400], 0x11, "the decoy survives");
    assert_eq!(cpu.pc, 0x0202);
    assert_flags(&cpu, false, false, true, true, "STA (zp),Y moves no flags");
}

// ----- Build 18: lookup tables -----

#[test]
fn lda_absolute_y_loads_a_and_sets_flags() {
    let (mut cpu, mut bus) = setup(&[0xB9, 0x00, 0x03, 0xB9, 0x01, 0x03, 0xB9, 0xFF, 0x03]);
    cpu.p = StatusFlag::Carry as u8;
    cpu.y = 0x05;
    bus.memory[0x0300] = 0xEE; // decoy
    bus.memory[0x0305] = 0x44;
    bus.memory[0x0306] = 0x00;
    bus.memory[0x0404] = 0x80; // $03FF + 5 crosses into page $04
    cpu.step(&mut bus);
    assert_eq!((cpu.a, cpu.pc), (0x44, 0x0203));
    assert_flags(&cpu, false, false, false, true, "LDA $0300,Y");
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00);
    assert_flags(&cpu, false, false, true, true, "LDA $0301,Y");
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x80);
    assert_flags(&cpu, true, false, false, true, "LDA $03FF,Y across a page");
}

#[test]
fn ldx_absolute_y_loads_x_and_sets_flags() {
    let (mut cpu, mut bus) = setup(&[0xBE, 0x00, 0x03, 0xBE, 0x01, 0x03, 0xBE, 0x02, 0x03]);
    cpu.p = StatusFlag::Carry as u8;
    cpu.y = 0x05;
    cpu.x = 0x02; // X differs from Y: an arm indexing by X reads the wrong byte
    bus.memory[0x0302..0x0308].copy_from_slice(&[0xEE, 0xEE, 0xEE, 0x44, 0x00, 0x80]);
    cpu.step(&mut bus);
    assert_eq!((cpu.x, cpu.pc), (0x44, 0x0203));
    assert_flags(&cpu, false, false, false, true, "LDX $0300,Y");
    cpu.step(&mut bus);
    assert_eq!(cpu.x, 0x00);
    assert_flags(&cpu, false, false, true, true, "LDX $0301,Y");
    cpu.step(&mut bus);
    assert_eq!(cpu.x, 0x80);
    assert_flags(&cpu, true, false, false, true, "LDX $0302,Y");
}

#[test]
fn ldy_absolute_x_loads_y_and_sets_flags() {
    let (mut cpu, mut bus) = setup(&[0xBC, 0x00, 0x03, 0xBC, 0x01, 0x03, 0xBC, 0x02, 0x03]);
    cpu.p = StatusFlag::Carry as u8;
    cpu.x = 0x05;
    cpu.y = 0x02;
    bus.memory[0x0302..0x0308].copy_from_slice(&[0xEE, 0xEE, 0xEE, 0x44, 0x00, 0x80]);
    cpu.step(&mut bus);
    assert_eq!((cpu.y, cpu.pc), (0x44, 0x0203));
    assert_flags(&cpu, false, false, false, true, "LDY $0300,X");
    cpu.step(&mut bus);
    assert_eq!(cpu.y, 0x00);
    assert_flags(&cpu, false, false, true, true, "LDY $0301,X");
    cpu.step(&mut bus);
    assert_eq!(cpu.y, 0x80);
    assert_flags(&cpu, true, false, false, true, "LDY $0302,X");
}

// ----- Build 21: tiny interactive monitor -----

#[test]
fn lda_absolute_loads_a_and_sets_flags() {
    let (mut cpu, mut bus) = setup(&[0xAD, 0x34, 0x12, 0xAD, 0x35, 0x12, 0xAD, 0x36, 0x12]);
    cpu.p = StatusFlag::Carry as u8;
    bus.memory[0x1234..0x1237].copy_from_slice(&[0x44, 0x00, 0x80]);
    cpu.step(&mut bus);
    assert_eq!((cpu.a, cpu.pc), (0x44, 0x0203));
    assert_flags(&cpu, false, false, false, true, "LDA $1234");
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00);
    assert_flags(&cpu, false, false, true, true, "LDA $1235");
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x80);
    assert_flags(&cpu, true, false, false, true, "LDA $1236");
}

/// Build 21: the console input port. These run on `ConsoleBus`, the only
/// bus that has one.
mod console_input {
    use crate::{Chip6502, ConsoleBus};

    /// `LDA $F000; STA $0090; JMP` to self, at `$0200`.
    const READ_PORT: [u8; 8] = [0xAD, 0x00, 0xF0, 0x85, 0x90, 0x4C, 0x05, 0x02];

    #[test]
    fn input_port_consumes_queued_byte() {
        let mut cpu = Chip6502::new();
        let mut bus = ConsoleBus::new();
        bus.ram.memory[0x0200..0x0208].copy_from_slice(&READ_PORT);
        bus.queue_input_byte(0x42);
        cpu.pc = 0x0200;
        cpu.step(&mut bus);
        cpu.step(&mut bus);
        assert_eq!(bus.ram.memory[0x0090], 0x42);
        assert!(bus.input_queue.is_empty());
    }

    #[test]
    fn input_port_empty_queue_reads_zero() {
        let mut cpu = Chip6502::new();
        let mut bus = ConsoleBus::new();
        bus.ram.memory[0x0200..0x0208].copy_from_slice(&READ_PORT);
        bus.ram.memory[0x0090] = 0xFF;
        cpu.pc = 0x0200;
        cpu.step(&mut bus);
        cpu.step(&mut bus);
        assert_eq!(
            bus.ram.memory[0x0090], 0x00,
            "the sentinel the WAIT loop spins on"
        );
    }
}

/// Build 3: the disassembler reproduces the trace the book prints for
/// Build 2's program, line for line.
mod disassembler {
    use crate::{FlatBus, disassemble_at};

    #[test]
    fn disassemble_at_matches_build_3_trace() {
        let mut bus = FlatBus::new();
        bus.memory[0x0200..0x0215].copy_from_slice(&[
            0xA2, 0x00, 0x8A, 0x18, 0x69, 0x30, 0x20, 0x11, 0x02, 0xE8, 0xE0, 0x0A, 0xD0, 0xF4,
            0x4C, 0x0E, 0x02, 0x8D, 0x01, 0xF0, 0x60,
        ]);
        let expected = [
            (0x0200, "0200: LDX #$00 (Immediate)"),
            (0x0202, "0202: TXA (Implied)"),
            (0x0203, "0203: CLC (Implied)"),
            (0x0204, "0204: ADC #$30 (Immediate)"),
            (0x0206, "0206: JSR $0211 (Absolute)"),
            (0x0211, "0211: STA $F001 (Absolute)"),
            (0x0214, "0214: RTS (Implied)"),
            (0x0209, "0209: INX (Implied)"),
            (0x020A, "020A: CPX #$0A (Immediate)"),
            (0x020C, "020C: BNE $0202 (Relative)"),
        ];
        for (pc, line) in expected {
            assert_eq!(disassemble_at(pc, &mut bus), line);
        }
    }

    #[test]
    fn unknown_opcode_disassembles_as_placeholder() {
        let mut bus = FlatBus::new();
        bus.memory[0x0300] = 0x02; // no 6502 instruction
        assert_eq!(disassemble_at(0x0300, &mut bus), "0300: ??? (Implied)");
    }
}
