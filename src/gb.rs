use crate::cpu::{CPU, Interrupt};
use crate::memory::Memory;
use std::fs::File;
use std::ffi::CStr;
use memmap::MmapOptions;

pub struct GB {
    pub rom_path: String,
    pub rom_title: String,
    cpu: CPU,
}

impl GB {
    pub fn with_rom(path: &str) -> GB {
        let rom_file = File::open(path).unwrap();
        let rom = unsafe { MmapOptions::new().map(&rom_file).unwrap() };
        let title_ptr = &rom[0x0134..0x144];
        GB {
            rom_path: path.to_string(),
            rom_title: unsafe { CStr::from_ptr(title_ptr.as_ptr() as *const i8) }
                .to_str().unwrap().to_owned(),
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

    pub fn reset(&mut self) {
        *self.cpu.mem.reg_lcdc() = 0x91;
        self.cpu.mem.data[0xff47] = 0xfc;
        self.cpu.mem.data[0xff48] = 0xff;
        self.cpu.mem.data[0xff49] = 0xff;
    }
}