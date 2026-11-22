// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! Conformance tests: Klaus Dormann's 6502 functional, decimal and interrupt suites.
//!
//! These are Build 26's acceptance tests, judged by programs somebody else
//! wrote. The suites are GPL-3.0 (the decimal test is Bruce Clark's, public
//! domain) and are not part of this repository: each test reads its image
//! at run time from the directory named by `CHIP6502_TEST_ROMS`, or from
//! `test_roms/` beside `Cargo.toml` if that is unset, and is `#[ignore]`d so
//! a plain `cargo test` never needs them. Run them in release:
//!
//! ```text
//! CHIP6502_TEST_ROMS=/path/to/roms cargo test --release -p chip6502_core klaus -- --ignored
//! ```
//!
//! All three run on `FlatBus`, so every one of the 64 KB behaves as RAM, and
//! none calls `reset()`: each sets PC to the suite's start address directly.
//!
//! Introduced in Build 26 — Conformance: finding and fixing compatibility
//! bugs; Build 27 pins each suite's cycle total.

use std::path::PathBuf;

use crate::{Chip6502, FlatBus};

/// Read one suite image, from `$CHIP6502_TEST_ROMS/<name>` or
/// `test_roms/<name>`, with a message that says how to obtain it.
fn read_rom(name: &str) -> Vec<u8> {
    let dir = std::env::var_os("CHIP6502_TEST_ROMS").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_roms"),
        PathBuf::from,
    );
    let path = dir.join(name);
    std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "cannot read {} ({error}).\n\
             Set CHIP6502_TEST_ROMS to the directory holding {name}, or put it in \
             chip6502_core/test_roms/ (git-ignored: the suites are GPL-3.0 and are \
             never committed). README.md says where to get them (Build 26).",
            path.display()
        )
    })
}

/// Build 26: the functional test traps on `JMP *`; `$3469` is success.
#[test]
#[ignore = "external binary required; set CHIP6502_TEST_ROMS and run with --ignored"]
#[rustfmt::skip]
#[expect(clippy::manual_assert, reason = "Build 26 prints the step ceiling as `if … { panic!(…) }`")]
fn fnl_klaus_dormann() {
    const START_PC:    u16 = 0x0400;   // confirm against .lst
    const SUCCESS_PC:  u16 = 0x3469;   // confirm against .lst
    const MAX_STEPS:   u64 = 100_000_000; // generous; the test is long

    let rom = read_rom("6502_functional_test.bin");
    assert_eq!(rom.len(), 0x10000, "expected a full 64 KB image");

    let mut cpu = Chip6502::new();
    let mut bus = FlatBus::new();   // NOT ConsoleBus — see Step 3: Klaus
                                    // needs the full 64 KB to behave as RAM

    // Load the whole image directly — it IS the 64 KB address space.
    bus.memory.copy_from_slice(&rom);
    cpu.pc = START_PC;

    let mut steps = 0u64;
    let started = std::time::Instant::now();
    loop {
        let pc_before = cpu.pc;
        cpu.step(&mut bus);
        steps += 1;

        if cpu.pc == pc_before {
            // Trapped. Either success or a failing sub-test.
            assert_eq!(
                cpu.pc, SUCCESS_PC,
                "Klaus trap at {:#06X} (not success {:#06X}) after {} steps — \
                 look this address up in 6502_functional_test.lst",
                cpu.pc, SUCCESS_PC, steps
            );
            let elapsed = started.elapsed();
            #[expect(clippy::cast_precision_loss, reason = "a rate for a human to read")]
            let mips = steps as f64 / elapsed.as_secs_f64() / 1e6;
            println!(
                "functional test passed: {steps} instructions, {} cycles, {elapsed:.2?} ({mips:.0} M instructions/s)",
                cpu.cycles
            );
            // Regression pins (Build 27, layer 2): any change to either total
            // is a change to explain. The book gives 30,646,177 instructions
            // and 96.2M cycles (Build 26, Independent verification).
            assert_eq!((steps, cpu.cycles), (30_646_177, 96_241_367), "functional test totals moved");
            break; // success
        }

        if steps >= MAX_STEPS {
            panic!("Klaus test did not trap within {MAX_STEPS} steps — \
                    likely an infinite loop from a wrong branch/jump, PC={:#06X}", cpu.pc);
        }
    }
}

/// Build 26: the decimal test ends on a `$DB` byte (65C02 `STP`) and
/// reports through the `ERROR` byte at `$000B`.
#[test]
#[ignore = "external binary required; set CHIP6502_TEST_ROMS and run with --ignored"]
#[rustfmt::skip]
#[expect(clippy::manual_assert, reason = "Build 26 prints the step ceiling as `if … { panic!(…) }`")]
fn fnl_klaus_dormann_decimal() {
    const START_PC:   u16 = 0x0200;
    const ERR_COUNT:  u16 = 0x000B;
    const STP_OPCODE: u8  = 0xDB;       // 65C02 STP, used by Klaus as halt marker
    const MAX_STEPS:  u64 = 100_000_000;

    let rom = read_rom("6502_decimal_test.bin");

    let mut cpu = Chip6502::new();
    let mut bus = FlatBus::new();   // pure RAM, as always for Klaus
    // Decimal test loads at $0200, not offset 0.
    bus.memory[0x0200..0x0200 + rom.len()].copy_from_slice(&rom);
    cpu.pc = START_PC;

    let mut steps = 0u64;
    loop {
        // Halt before executing the STP byte at DONE — if we step into it,
        // our cfg(test)-panicking `not_implemented` handler will kill the
        // test with an unhelpful "opcode $DB" panic.
        if bus.memory[cpu.pc as usize] == STP_OPCODE {
            let errors = bus.memory[ERR_COUNT as usize];
            assert_eq!(
                errors, 0,
                "BCD test: ERROR={} at end-of-test (PC=${:04X}, {} steps). \
                 0=pass, non-zero=fail.",
                errors, cpu.pc, steps
            );
            println!("decimal test passed: {steps} instructions, {} cycles", cpu.cycles);
            // Regression pins (Build 27, layer 2): any change to either total
            // is a change to explain. The instruction count is the book's.
            assert_eq!((steps, cpu.cycles), (14_464_187, 46_089_505), "decimal test totals moved");
            break;
        }
        cpu.step(&mut bus);
        steps += 1;
        if steps >= MAX_STEPS {
            panic!("decimal test did not reach STP within {MAX_STEPS} steps");
        }
    }
}

/// Build 26: the interrupt test drives IRQ and NMI through the plain-RAM
/// register at `$BFFC`, which the harness bridges to the CPU's interrupt
/// methods before every step.
#[test]
#[ignore = "external binary required; set CHIP6502_TEST_ROMS and run with --ignored"]
#[rustfmt::skip]
#[expect(clippy::manual_assert, reason = "Build 26 prints the step ceiling as `if … { panic!(…) }`")]
fn fnl_klaus_dormann_interrupts() {
    const START_PC:   u16 = 0x0400;
    const SUCCESS_PC: u16 = 0x06F5;
    const INT_CTRL:   u16 = 0xBFFC;
    const LOAD_AT:    usize = 0x000A;       // from .lst: "$000a through $ffff"
    const MAX_STEPS:  u64 = 100_000_000;

    let rom = read_rom("6502_interrupt_test.bin");
    assert_eq!(
        rom.len(), 0xFFFF - LOAD_AT + 1,
        "binary size {} doesn't match expected layout (0x{:04X}..0xFFFF)",
        rom.len(), LOAD_AT
    );

    let mut cpu = Chip6502::new();
    let mut bus = FlatBus::new();   // pure RAM, as always for Klaus
    bus.memory[LOAD_AT..LOAD_AT + rom.len()].copy_from_slice(&rom);
    cpu.pc = START_PC;

    // GOTCHA #1: the binary's $FF padding covers $BFFC, so the interrupt
    // control register loads as $FF — i.e. "both IRQ and NMI asserted".
    // On step 0 that fires a spurious NMI before any test code runs (the
    // prev/cur transition for NMI is 0 -> 1). Real hardware boots with
    // interrupt lines deasserted; mirror that by clearing $BFFC explicitly.
    bus.memory[INT_CTRL as usize] = 0;

    let mut steps = 0u64;
    let mut prev_int_ctrl: u8 = 0;
    loop {
        // Mask off bit 7 — I_filter = $7F reserves it for emulator-side
        // "diag stop" signaling; we don't use it.
        let ctrl = bus.memory[INT_CTRL as usize] & 0x7F;

        // IRQ is level-triggered: mirror the bit each step.
        if ctrl & 0x01 != 0 {
            cpu.request_irq();
        } else {
            cpu.clear_irq();
        }

        // GOTCHA #2: NMI is edge-triggered. Fire ONLY on the 0 -> 1
        // transition. Treating NMI as level-triggered fires it every step
        // the bit is set, which results in nested NMIs and an early trap.
        if (ctrl & 0x02) != 0 && (prev_int_ctrl & 0x02) == 0 {
            cpu.trigger_nmi();
        }
        prev_int_ctrl = ctrl;

        let pc_before = cpu.pc;
        cpu.step(&mut bus);
        steps += 1;

        if cpu.pc == pc_before {
            // Trapped on a self-jump — either the success macro at $06F5
            // or one of the trap_xx macros scattered through the test.
            assert_eq!(
                cpu.pc, SUCCESS_PC,
                "interrupt test trapped at ${:04X} (not success ${:04X}) after {} steps — \
                 look this address up in the .a65 source: it's a trap_xx macro, and the \
                 comment a few lines above identifies which interrupt scenario failed",
                cpu.pc, SUCCESS_PC, steps
            );
            println!("interrupt test passed: {steps} instructions, {} cycles", cpu.cycles);
            // Regression pins (Build 27, layer 2): any change to either total
            // is a change to explain. The instruction count is the book's.
            assert_eq!((steps, cpu.cycles), (1_050, 3_016), "interrupt test totals moved");
            break;
        }
        if steps >= MAX_STEPS {
            panic!("interrupt test did not trap within {MAX_STEPS} steps");
        }
    }
}
