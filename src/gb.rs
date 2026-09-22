use crate::cpu::{Interrupt, CPU};
use crate::memory_types::{DMGPalette, LCDControl};
use crate::ppu::PPU;
use crate::memory::Memory;

pub struct GB<'a> {
    pub mem: Memory<'a>,
    pub cpu: CPU,
    ppu: PPU,
}

impl GB<'_> {
    pub fn with_rom_buffer(rom_buffer: &[u8]) -> GB<'_> {
        GB {
            mem: Memory::with_rom_buffer(rom_buffer),
            cpu: CPU::new(),
            ppu: PPU::new(),
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
        let (redraw, vblank) = self.ppu.step(&mut self.mem, buf, cycles);

        if vblank {
            self.cpu.set_interrupt(&mut self.mem, Interrupt::VBlank);
        }

        (cycles, redraw)
    }

    pub fn reset(&mut self) {
        self.cpu.reg.a = 0x01;
        self.cpu.reg.f = 0xb0;
        self.cpu.reg.c = 0x13;
        self.cpu.reg.e = 0xd8;
        self.cpu.reg.h = 0x01;
        self.cpu.reg.l = 0x4d;

        self.mem.joypad_states[0] = 0x0f;
        self.mem.joypad_states[1] = 0x0f;

        self.mem.reg_mut().lcd_control = LCDControl::from_bits(0x91);
        self.mem.reg_mut().bg_palette_data = DMGPalette::from_bits(0xfc);
        self.mem.reg_mut().object_palette_0 = DMGPalette::from_bits(0xff);
        self.mem.reg_mut().object_palette_1 = DMGPalette::from_bits(0xff);
    }
}

#[cfg(test)]
mod tests {
    use super::GB;

    #[test]
    fn reset_initializes_hl() {
        let rom = [0; 0x150];
        let mut gb = GB::with_rom_buffer(&rom);

        gb.reset();

        assert_eq!(gb.cpu.hl(), 0x014d);
    }
}
