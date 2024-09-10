use crate::cpu::{Interrupt, CPU};
use crate::gpu::GPU;
use crate::memory::Memory;
use memmap::MmapOptions;
use std::ffi::CStr;
use std::fs::File;

pub struct GB {
    pub rom_path: String,
    pub rom_title: String,
    pub mem: Memory,
    pub cpu: CPU,
    gpu: GPU,
}

impl GB {
    pub fn with_rom(path: &str) -> GB {
        let rom_file = File::open(path).unwrap();
        let rom = unsafe { MmapOptions::new().map(&rom_file).unwrap() };
        let title_ptr = &rom[0x0134..0x144];
        GB {
            rom_path: path.to_string(),
            rom_title: unsafe { CStr::from_ptr(title_ptr.as_ptr() as *const i8) }
                .to_str()
                .unwrap()
                .to_owned(),
            mem: Memory::with_rom(rom),
            cpu: CPU::new(),
            gpu: GPU::new(),
        }
    }

    pub fn set_joypad(&mut self, directional: usize, button: u8) {
        let mask = 1 << button;
        if self.mem.joypad_states[directional] & mask != 0 {
            self.mem.joypad_states[directional] &= !mask;
            self.cpu.set_interrupt(&mut self.mem, Interrupt::JoyPad);
        }
    }

    pub fn unset_joypad(&mut self, directional: usize, button: u8) {
        self.mem.joypad_states[directional] |= 1 << button;
    }

    pub fn step(&mut self, buf: &mut [u8]) -> (u16, bool) {
        let cycles = self.cpu.step(&mut self.mem);
        let (redraw, vblank) = self.gpu.step(&mut self.mem, cycles);

        if vblank {
            self.cpu.set_interrupt(&mut self.mem, Interrupt::VBlank);
        }

        if redraw {
            self.gpu.draw_screen(&mut self.mem, buf);
        }

        (cycles, redraw)
    }

    pub fn reset(&mut self) {
        self.cpu.reg.a = 0x01;
        self.cpu.reg.f = 0xb0;
        self.cpu.reg.c = 0x13;
        self.cpu.reg.e = 0xd8;
        self.cpu.reg.h = 0x01;
        self.cpu.reg.h = 0x4d;

        self.mem.joypad_states[0] = 0x0f;
        self.mem.joypad_states[1] = 0x0f;

        *self.mem.reg_lcdc() = 0x91;
        self.mem.data[0xff47] = 0xfc;
        self.mem.data[0xff48] = 0xff;
        self.mem.data[0xff49] = 0xff;
    }
}
