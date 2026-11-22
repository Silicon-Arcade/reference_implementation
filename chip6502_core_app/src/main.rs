// SPDX-FileCopyrightText: 2026 Jason Hogg
// SPDX-License-Identifier: Apache-2.0
// AI use: see AI_POLICY.md
//
//! The 6502 emulator application: Build 21's monitor in a window.
//!
//! An egui window around the core: keys typed into the window are queued at
//! the monitor's `$F000` input port, bytes the program writes to `$F001`
//! appear in the transcript, and a control bar runs, pauses and resets the
//! CPU. The design is egui's immediate mode: all state lives in `AppState`,
//! and `update` redraws the whole window from it every frame, after stepping
//! the CPU up to 2,000 instructions on the UI thread. Nothing in the core
//! changes for this.
//!
//! Introduced in Build 29 — The Emulator Application; Appendix F explains the egui it uses.

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::collections::VecDeque;

use chip6502_core::{Chip6502, ConsoleBus};
use eframe::egui;

/// Everything the application keeps from one frame to the next.
struct AppState {
    cpu: Chip6502,
    bus: ConsoleBus,
    transcript: String,          // accumulated output to display
    output_consumed: usize,      // bytes of the bus output log already shown
    auto_run: bool,              // is the CPU currently auto-stepping?
    pending_input: VecDeque<u8>, // keys captured but not yet fed to CPU
}

impl Default for AppState {
    fn default() -> Self {
        debug_assert_eq!(MONITOR_PROGRAM.len(), 179);
        let mut cpu = Chip6502::new();
        let mut bus = ConsoleBus::new();

        // Load the monitor at $0200, point the reset vector at it, then
        // reset so SP and flags initialize correctly (the monitor uses
        // JSR/RTS). This is Chapter 4's main.rs ritual again.
        for (i, &byte) in MONITOR_PROGRAM.iter().enumerate() {
            bus.ram.memory[0x0200 + i] = byte;
        }
        bus.ram.memory[0xFFFC] = 0x00;
        bus.ram.memory[0xFFFD] = 0x02;
        cpu.reset(&mut bus);

        Self {
            cpu,
            bus,
            transcript: String::new(),
            output_consumed: 0,
            auto_run: false, // the Run button starts it
            pending_input: VecDeque::new(),
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- Capture keyboard input ---
        ctx.input(|i| {
            for event in &i.events {
                match event {
                    egui::Event::Text(text) => {
                        // Each pressed key produces a `Text` event with the
                        // typed character. Push each byte.
                        for b in text.bytes() {
                            self.pending_input.push_back(b);
                        }
                    }
                    egui::Event::Key { key: egui::Key::Enter, pressed: true, .. } => {
                        self.pending_input.push_back(b'\n');
                    }
                    _ => {}
                }
            }
        });

        // --- Feed pending input into the bus's MMIO input queue ($F000) ---
        while let Some(byte) = self.pending_input.pop_front() {
            self.bus.queue_input_byte(byte);
        }

        // --- Step the CPU (auto-run mode) ---
        if self.auto_run {
            self.run_frame();
            ctx.request_repaint(); // tell egui to immediately redraw
        }

        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Run / Pause").clicked() {
                    self.auto_run = !self.auto_run;
                }
                if ui.button("Reset").clicked() {
                    *self = Self::default();
                }
                if ui.button("Clear transcript").clicked() {
                    self.transcript.clear();
                    self.output_consumed = self.bus.output_log.len();
                }
                ui.label(format!(
                    "Status: {}",
                    if self.auto_run { "Running" } else { "Paused" }
                ));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Transcript");
            ui.label("Click in the window, then type to interact with the monitor.");
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.monospace(&self.transcript);
                });
        });
    }
}

impl AppState {
    /// Run up to 2,000 instructions, stop auto-running if PC stops moving,
    /// and drain the new `$F001` output into the transcript.
    fn run_frame(&mut self) {
        // Run up to 2,000 instructions per frame: an instruction budget,
        // not a cycle budget (Build 27 outlines the cycle-based version).
        let mut stop_note = None;
        for _ in 0..2000 {
            let pc_before = self.cpu.pc;
            self.cpu.step(&mut self.bus);
            if self.cpu.pc == pc_before {
                // PC did not move: the program halted itself (JMP to its
                // own address), or it hit an opcode the core does not model.
                stop_note = self
                    .cpu
                    .last_unimplemented_opcode()
                    .filter(|&(_, pc)| pc == pc_before)
                    .map(|(op, pc)| {
                        format!("\n[stopped: opcode ${op:02X} at ${pc:04X} is not emulated]\n")
                    });
                self.auto_run = false;
                break;
            }
        }

        // Drain new output bytes ($F001 log) into the transcript.
        let log_len = self.bus.output_log.len();
        if log_len > self.output_consumed {
            let new_bytes = &self.bus.output_log[self.output_consumed..log_len];
            self.transcript
                .push_str(&String::from_utf8_lossy(new_bytes));
            self.output_consumed = log_len;
        }
        // After the drain, so the note follows the output that led to it.
        if let Some(note) = stop_note {
            self.transcript.push_str(&note);
        }
    }
}

// The Build 21 monitor program byte stream, 179 bytes.
#[rustfmt::skip]
const MONITOR_PROGRAM: &[u8] = &[
    // ───── Banner @ $0200 ─────
    0xA9, 0x87, 0x85, 0x80, 0xA9, 0x02, 0x85, 0x81,            // BANNER ptr
    0x20, 0x7A, 0x02,                                          // JSR PRINT_STR

    // ───── LOOP: prompt @ $020B ─────
    0xA9, 0x3E, 0x8D, 0x01, 0xF0,                              // '>'
    0xA9, 0x20, 0x8D, 0x01, 0xF0,                              // ' '

    // ───── WAIT @ $0215 ─────
    0xAD, 0x00, 0xF0,                                          // LDA $F000
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

fn main() -> Result<(), eframe::Error> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_title("6502 Processor"),
        ..Default::default()
    };
    eframe::run_native(
        "6502 Processor",
        opts,
        Box::new(|_cc| Ok(Box::new(AppState::default()))),
    )
}

#[cfg(test)]
mod tests {
    //! Build 29: the app's own logic, driven without a window.

    use super::AppState;

    #[test]
    fn monitor_session_without_a_window() {
        let mut app = AppState { auto_run: true, ..AppState::default() };
        app.run_frame(); // banner, then the WAIT loop spins out the budget
        assert!(app.transcript.starts_with("MON v1\n?=help\n> "));

        app.bus.synthesize_queue_input(b"AQ");
        app.run_frame();
        assert!(app.transcript.ends_with("> A\ngot A\n> Q\nbye\n"));
        assert!(!app.auto_run, "the JMP-self halt stops auto-run");
    }

    #[test]
    fn an_unmodeled_opcode_stops_the_run_with_a_note() {
        let mut app = AppState::default();
        app.bus.ram.memory[0x0200] = 0x02; // an opcode the core does not model
        app.auto_run = true;
        app.run_frame();
        // This crate's tests build the core normally, so its production path runs.
        assert!(
            app.transcript
                .contains("[stopped: opcode $02 at $0200 is not emulated]")
        );
        assert!(!app.auto_run);
    }
}
