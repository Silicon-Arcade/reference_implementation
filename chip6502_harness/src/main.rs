// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! Interactive harness: load a raw 6502 image at $1000 and play it over stdin.
//!
//! Type keys and press Enter to send them. A line of digits is a move
//! and is committed with CR ($0D); a blank line sends a bare CR.
//!
//! Build 30 uses it to run Peter Jennings's Microchess, which reaches this
//! program only as a file the reader assembles and keeps out of the
//! repository; nothing here contains a byte of it. The `Console` wrapper is
//! the design point: a read of `$F000` with nothing queued is the program
//! saying "your turn", so the harness stops stepping exactly there.
//!
//! What an image must do to run here: assemble to load and start at `$1000`
//! (the harness copies it there and points the reset vector at it), write
//! its output to `$F001`, and take its input by polling `$F000` rather than
//! by waiting on an interrupt. An image that never reads `$F000` runs to the
//! instruction ceiling instead of handing you a turn. Build 30's five
//! "fit it to your emulator" changes are what bring Microchess to this
//! contract.
//!
//! Introduced in Build 30 — Case Study: Microchess.

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use chip6502_core::{Bus, Chip6502, ConsoleBus, INPUT_PORT};
use std::io::{BufRead, Write};

/// A `ConsoleBus` that notices the program reading `$F000` with nothing
/// queued. That read is the program saying "I am waiting for you", and
/// it is the only reliable signal that a turn is over.
struct Console {
    inner: ConsoleBus,
    waiting: bool,
}

impl Bus for Console {
    fn memory_read(&mut self, addr: u16) -> u8 {
        if addr == INPUT_PORT && self.inner.input_queue.is_empty() {
            self.waiting = true;
        }
        self.inner.memory_read(addr)
    }
    fn memory_write(&mut self, addr: u16, value: u8) {
        self.inner.memory_write(addr, value);
    }
}

/// Run the program until it waits for keyboard input or stops itself,
/// then return. If it runs 100 million instructions without doing either,
/// or hits an opcode the core does not support, print why and exit.
fn run(cpu: &mut Chip6502, bus: &mut Console) {
    bus.waiting = false;
    for _ in 0..100_000_000u64 {
        let pc_before = cpu.pc;
        cpu.step(bus);
        if bus.waiting {
            return;
        }
        if cpu.pc == pc_before {
            // PC did not move: the program stopped itself (Q ends in a
            // jump to itself), or the core met an opcode it does not support.
            if let Some((op, pc)) = cpu.last_unimplemented_opcode() {
                eprintln!("\n[stopped: opcode ${op:02X} at ${pc:04X} is not emulated]");
                std::process::exit(1);
            }
            return;
        }
    }
    eprintln!("\n[stopped: 100,000,000 instructions without waiting for input]");
    std::process::exit(1);
}

/// Print whatever the program has emitted since we last looked.
fn show(bus: &Console, shown: &mut usize) {
    print!(
        "{}",
        String::from_utf8_lossy(&bus.inner.output_log[*shown..])
    );
    *shown = bus.inner.output_log.len();
    std::io::stdout().flush().ok();
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: harness <image.bin>");
    let image = std::fs::read(&path).expect("cannot read image");

    let mut cpu = Chip6502::new();
    let mut bus = Console { inner: ConsoleBus::new(), waiting: false };
    bus.inner.ram.memory[0x1000..0x1000 + image.len()].copy_from_slice(&image);
    bus.inner.ram.memory[0xFFFC] = 0x00;
    bus.inner.ram.memory[0xFFFD] = 0x10;
    cpu.reset(&mut bus);

    let mut shown = 0;
    run(&mut cpu, &mut bus); // boot, so the first board is up before you type
    show(&bus, &mut shown);

    for line in std::io::stdin().lock().lines() {
        let line = line.expect("stdin");
        let bytes = line.as_bytes();
        if bytes.is_empty() {
            bus.inner.queue_input_byte(0x0D); // bare Enter = CR, never LF
        } else {
            bus.inner.synthesize_queue_input(bytes);
            // A typed move is digits and commits itself with CR. Command
            // letters must NOT get one: Microchess reads $0D as "play the
            // move now in the register", which after C would play garbage.
            if bytes.iter().all(u8::is_ascii_digit) {
                bus.inner.queue_input_byte(0x0D);
            }
        }
        run(&mut cpu, &mut bus);
        show(&bus, &mut shown);
    }
}

#[cfg(test)]
mod tests {
    //! Build 30: structural tests. Microchess's output is not ours to pin
    //! byte for byte, so these assert structure, and read the board from RAM
    //! rather than from the rendered text. The image is read at test time
    //! from `MICROCHESS_BIN` and never stored in this repository.

    use super::Console;
    use chip6502_core::{Chip6502, ConsoleBus};

    /// Build 1's cap: a program that never polls must fail, not hang.
    const CEILING: u64 = 100_000_000;

    /// Why `run_until_waiting` stopped without the program waiting for input.
    #[derive(Debug, PartialEq, Eq)]
    enum Stop {
        Halted,
        Ceiling,
    }

    /// A CPU and console with `image` loaded at `$1000` and reset onto it.
    fn boot(image: &[u8]) -> (Chip6502, Console) {
        let mut cpu = Chip6502::new();
        let mut bus = Console { inner: ConsoleBus::new(), waiting: false };
        bus.inner.ram.memory[0x1000..0x1000 + image.len()].copy_from_slice(image);
        bus.inner.ram.memory[0xFFFC..=0xFFFD].copy_from_slice(&[0x00, 0x10]);
        cpu.reset(&mut bus);
        (cpu, bus)
    }

    /// `run` without `process::exit`: `Ok` when the program polls an empty
    /// input queue, `Err` when it halts or reaches the ceiling.
    fn run_until_waiting(cpu: &mut Chip6502, bus: &mut Console) -> Result<(), Stop> {
        bus.waiting = false;
        for _ in 0..CEILING {
            let pc_before = cpu.pc;
            cpu.step(bus);
            if bus.waiting {
                return Ok(());
            }
            if cpu.pc == pc_before {
                return Err(Stop::Halted);
            }
        }
        Err(Stop::Ceiling)
    }

    /// The Microchess image named by `MICROCHESS_BIN`.
    fn microchess() -> Vec<u8> {
        let path = std::env::var_os("MICROCHESS_BIN")
            .expect("set MICROCHESS_BIN to the microchess.bin you assembled (Build 30; README.md)");
        std::fs::read(&path).expect("cannot read MICROCHESS_BIN")
    }

    /// The last board drawn: its eight rank lines, `00` first.
    fn last_board(bus: &Console) -> Vec<String> {
        let text = String::from_utf8_lossy(&bus.inner.output_log).into_owned();
        let ranks: Vec<String> = text
            .lines()
            .filter(|line| line.starts_with('|') && line.len() == 27)
            .map(str::to_owned)
            .collect();
        ranks[ranks.len() - 8..].to_vec()
    }

    #[test]
    #[ignore = "needs the Microchess image; set MICROCHESS_BIN and run with --ignored"]
    fn it_boots() {
        let (mut cpu, mut bus) = boot(&microchess());
        assert_eq!(
            run_until_waiting(&mut cpu, &mut bus),
            Ok(()),
            "syskin must poll $F000"
        );
        assert_eq!(last_board(&bus).len(), 8, "eight rank lines");
    }

    #[test]
    #[ignore = "needs the Microchess image; set MICROCHESS_BIN and run with --ignored"]
    fn it_accepts_a_command() {
        let (mut cpu, mut bus) = boot(&microchess());
        run_until_waiting(&mut cpu, &mut bus).expect("boot");
        bus.inner.queue_input_byte(b'C');
        run_until_waiting(&mut cpu, &mut bus).expect("set up the board");
        let board = last_board(&bus);
        assert!(
            board[0].contains("WK") && board[0].contains("WQ"),
            "white back rank: {}",
            board[0]
        );
        assert_eq!(
            board[1].matches("WP").count(),
            8,
            "white pawns: {}",
            board[1]
        );
        assert_eq!(
            board[6].matches("BP").count(),
            8,
            "black pawns: {}",
            board[6]
        );
        assert!(
            board[7].contains("BK") && board[7].contains("BQ"),
            "black back rank: {}",
            board[7]
        );
    }

    #[test]
    #[ignore = "needs the Microchess image; set MICROCHESS_BIN and run with --ignored"]
    fn it_plays() {
        let (mut cpu, mut bus) = boot(&microchess());
        run_until_waiting(&mut cpu, &mut bus).expect("boot");
        bus.inner.queue_input_byte(b'C');
        run_until_waiting(&mut cpu, &mut bus).expect("set up the board");

        // BOARD is thirty-two bytes at $50, one square per piece.
        let before = bus.inner.ram.memory[0x50..0x70].to_vec();
        bus.inner.queue_input_byte(b'P');
        assert_eq!(
            run_until_waiting(&mut cpu, &mut bus),
            Ok(()),
            "back at the input poll"
        );
        let after = &bus.inner.ram.memory[0x50..0x70];
        let moved: Vec<usize> = (0..32).filter(|&i| before[i] != after[i]).collect();
        assert_eq!(moved.len(), 1, "exactly one piece should have moved");
        assert_ne!(
            after[moved[0]], 0xCC,
            "and it should not have been captured"
        );
    }

    #[test]
    fn the_instruction_ceiling_holds() {
        // Two jumps that bounce forever: PC always moves, $F000 is never read.
        let image = [0x4C, 0x03, 0x10, 0x4C, 0x00, 0x10];
        let (mut cpu, mut bus) = boot(&image);
        assert_eq!(run_until_waiting(&mut cpu, &mut bus), Err(Stop::Ceiling));
    }
}
