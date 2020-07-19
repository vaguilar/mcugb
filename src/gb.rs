use crate::cpu::CPU;
use crate::memory::Memory;
use std::fs::File;
use memmap::MmapOptions;

pub struct GB {
    cpu: CPU,
    mem: Memory,
}

impl GB {
    pub fn with_rom(path: &str) -> GB {
        let rom_file = File::open(path).unwrap();
        let rom = unsafe { MmapOptions::new().map(&rom_file).unwrap() };
        GB {
            cpu: CPU::new(),
            mem: Memory::with_rom(rom),
        }
    }

    pub fn step(&self) {
    }

    pub fn set_joypad(&self, directional: u8, button: u8) {
        // let mask = (1 << button);
        // if (cpu_joypad_states[directional] & mask) {
        //     cpu_joypad_states[directional] &= ~mask;
        //     self.cpu.interrupt(INT_JOYPAD);
        // }
    }

    pub fn unset_joypad(&self, directional: u8, button: u8) {
        // cpu_joypad_states[directional] |= 1 << button;
    }
}