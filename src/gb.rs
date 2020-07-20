use crate::cpu::{CPU, Interrupt};
use crate::memory::Memory;
use std::fs::File;
use memmap::MmapOptions;

pub struct GB {
    cpu: CPU,
}

impl GB {
    pub fn with_rom(path: &str) -> GB {
        let rom_file = File::open(path).unwrap();
        let rom = unsafe { MmapOptions::new().map(&rom_file).unwrap() };
        GB {
            cpu: CPU::new(Memory::with_rom(rom)),
        }
    }

    pub fn set_joypad(&mut self, directional: usize, button: u8) {
        let mask = 1 << button;
        if self.cpu.mem.joypad_states[directional] & mask != 0 {
            self.cpu.mem.joypad_states[directional] &= !mask;
            self.cpu.set_interrupt(Interrupt::JoyPad);
        }
    }

    pub fn unset_joypad(&mut self, directional: usize, button: u8) {
        self.cpu.mem.joypad_states[directional] |= 1 << button;
    }

    pub fn step(&mut self) {
        self.cpu.step();
    }
}