// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! Build 28: the production arm of `not_implemented`.
//!
//! Integration tests link the library as an application would, without
//! `cfg(test)`, so this is the one test that reaches the normal-build
//! behavior the no-panic rule depends on.
//!
//! Introduced in Build 28 — Error Handling: Making the Core's Failure Model Explicit.

// tests/unimplemented_opcode.rs — sees the library as a normal build does
use chip6502_core::{Chip6502, FlatBus};

#[test]
fn production_core_records_unimplemented_opcode_without_panicking() {
    let mut bus = FlatBus::new();
    let mut cpu = Chip6502::new();
    bus.memory[0x0200] = 0x02; // undocumented: a dash in Appendix E
    cpu.pc = 0x0200;

    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x0200); // parked, not advanced
    assert_eq!(cpu.last_unimplemented_opcode(), Some((0x02, 0x0200)));

    cpu.reset(&mut bus);
    assert_eq!(cpu.last_unimplemented_opcode(), None);
}
