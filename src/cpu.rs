use crate::memory::Memory;

static TAC_SELECT: [u16; 4] = [1024, 16, 64, 256];

static BIT11: u16 = 1 << 11;
static BIT15: u16 = 1 << 15;

static FLAG_Z: u8 = 0x80;
static FLAG_N: u8 = 0x40;
static FLAG_H: u8 = 0x20;
static FLAG_C: u8 = 0x10;

pub enum Interrupt {
    VBlank = 0x01,
    LCDC = 0x02,
    Timer = 0x04,
    Serial = 0x08,
    JoyPad = 0x10,
}

pub struct CPU {
    pub pc: u16,
    pub sp: u16,
    pub reg: Registers,

    pub halt: bool,
    pub interrupts: bool,
    pub interrupt_enable: bool,
    pub interrupt_flag: bool,
    pub timer_cycles: u16,
    pub divider_cycles: u16,
}

#[derive(Default)]
pub struct Registers {
    f: u8,
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            pc: 0x0100,
            sp: 0xffff,
            reg: Default::default(),
            halt: false,
            interrupts: false,
            interrupt_enable: false,
            interrupt_flag: false,
            timer_cycles: 0,
            divider_cycles: 0,
        }
    }

    // Wide registers

    pub fn af(&self) -> u16 {
        ((self.reg.a as u16) << 8) | (self.reg.f as u16)
    }

    pub fn set_af(&mut self, val: u16) {
        self.reg.a = val as u8;
        self.reg.f = (val >> 8) as u8;
    }

    pub fn bc(&self) -> u16 {
        ((self.reg.b as u16) << 8) | (self.reg.c as u16)
    }

    pub fn set_bc(&mut self, val: u16) {
        self.reg.b = val as u8;
        self.reg.c = (val >> 8) as u8;
    }

    pub fn de(&self) -> u16 {
        ((self.reg.d as u16) << 8) | (self.reg.e as u16)
    }

    pub fn set_de(&mut self, val: u16) {
        self.reg.d = val as u8;
        self.reg.e = (val >> 8) as u8;
    }

    pub fn hl(&self) -> u16 {
        ((self.reg.h as u16) << 8) | (self.reg.l as u16)
    }

    pub fn set_hl(&mut self, val: u16) {
        self.reg.h = val as u8;
        self.reg.l = (val >> 8) as u8;
    }

    // Flags

    pub fn set_flag(&self, flag: u8, val: bool) {}

    pub fn nz(&self) -> bool {
        false
    }

    pub fn z(&self) -> bool {
        false
    }

    pub fn nc(&self) -> bool {
        false
    }

    pub fn c(&self) -> bool {
        false
    }

    pub fn set_interrupt(&mut self, mem: &mut Memory, interrupt: Interrupt) {
        mem.data[0xff0f] |= interrupt as u8;
    }

    pub fn unset_interrupt(&mut self, mem: &mut Memory, interrupt: Interrupt) {
        mem.data[0xff0f] &= !(interrupt as u8);
    }

    pub fn interrupt_reg(&self, mem: &mut Memory) -> u8 {
        mem.data[0xffff]
    }

    // Stack

    pub fn push_stack(&mut self, mem: &mut Memory, value: u16) {
        self.sp -= 2;
        mem.write16(self.sp, value);
    }

    pub fn pop_stack(&mut self, mem: &mut Memory) -> u16 {
        let result = mem.read16(self.sp);
        self.sp += 2;
        result
    }

    // Memory

    pub fn fetch8(&mut self, mem: &mut Memory) -> u8 {
        let result = mem.read8(self.pc);
        self.pc += 1;
        result
    }

    pub fn fetch16(&mut self, mem: &mut Memory) -> u16 {
        let result = mem.read16(self.pc);
        eprintln!("FETCH16 Got 0x{:04X} at PC=0x{:04x}", result, self.pc);
        self.pc += 2;
        result
    }

    // Execute

    pub fn step(&mut self, mem: &mut Memory) -> u16 {
        let op = self.fetch8(mem);
        eprintln!("Found 0x{:02X} at PC=0x{:04x}", op, self.pc);

        let mut cycles: u16 = 0;

        if op == 0xcb {
            let op2 = self.fetch8(mem);
            cycles = self.execute_cb(op2);
        } else {
            cycles = self.execute(mem, op);
        }

        let interrupt_flag = &mem.data[0xff0f];
        let interrupt_enable = &mem.data[0xffff];
        if self.interrupts {
            if *interrupt_enable & Interrupt::VBlank as u8 & *interrupt_flag != 0 {
                self.interrupts = false;
                self.push_stack(mem, self.pc);
                self.pc = 0x0040;
                self.unset_interrupt(mem, Interrupt::VBlank);
            } else if *interrupt_enable & Interrupt::LCDC as u8 & *interrupt_flag != 0 {
                self.interrupts = false;
                self.push_stack(mem, self.pc);
                self.pc = 0x0048;
                self.unset_interrupt(mem, Interrupt::LCDC);
            } else if *interrupt_enable & Interrupt::Timer as u8 & *interrupt_flag != 0 {
                self.interrupts = false;
                self.push_stack(mem, self.pc);
                self.pc = 0x0050;
                self.unset_interrupt(mem, Interrupt::Timer);
            } else if *interrupt_enable & Interrupt::Serial as u8 & *interrupt_flag != 0 {
                self.interrupts = false;
                self.push_stack(mem, self.pc);
                self.pc = 0x0058;
                self.unset_interrupt(mem, Interrupt::Serial);
            } else if *interrupt_enable & Interrupt::JoyPad as u8 & *interrupt_flag != 0 {
                self.interrupts = false;
                self.push_stack(mem, self.pc);
                self.pc = 0x0060;
                self.unset_interrupt(mem, Interrupt::JoyPad);
            }
        }

        /* update timer */
        self.update_timer(mem, cycles);
        cycles
    }

    fn update_timer(&mut self, mem: &mut Memory, cycles: u16) {
        let tac = *mem.reg_tac();

        let mode: usize = tac as usize & 0b00000011;
        let timer_on: u8 = tac & 0b00000010;

        if timer_on != 0 {
            self.timer_cycles += cycles;
            if self.timer_cycles > TAC_SELECT[mode] {
                self.timer_cycles -= TAC_SELECT[mode];
                *mem.reg_tima() += 1;

                if *mem.reg_tima() == 0 {
                    *mem.reg_tima() = *mem.reg_tma();
                    self.set_interrupt(mem, Interrupt::Timer);
                }
            }
        }

        self.divider_cycles += cycles;
        if self.divider_cycles > 255 {
            self.divider_cycles -= 255;
            *mem.reg_div() += 1;
        }
    }

    fn execute(&mut self, mem: &mut Memory, op: u8) -> u16 {
        let mut byte: u8;
        let mut immediate: u16;
        let mut result4: u8;
        let mut result: u8;
        let mut addr: u16;

        match op {
            0x00 => 0,
            0x01 => {
                // ld bc, nn
                let val = self.fetch16(mem);
                self.set_bc(val);
                // if (DEBUG) sprintf(instruction_str, "ld bc, $%04x", immediate16);
                12
            }
            0x02 => {
                // ld (bc), a
                mem.write8(self.bc(), self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "ld (bc), a");
                8
            }
            0x03 => {
                // inc bc
                self.reg.c += 1;
                if self.reg.c == 0x00 {
                    self.reg.b += 1;
                }
                // if (DEBUG) sprintf(instruction_str, "inc bc");
                8
            }
            0x04 => {
                // inc b
                inc8(&self.reg.b);
                // if (DEBUG) sprintf(instruction_str, "inc b");
                4
            }
            0x05 => {
                // dec b
                dec8(&self.reg.b);
                // if (DEBUG) sprintf(instruction_str, "dec b");
                4
            }
            0x06 => {
                // ld b, n
                self.reg.b = self.fetch8(mem);
                // if (DEBUG) sprintf(instruction_str, "ld b, $%02x", self.reg.b);
                8
            }
            0x07 => {
                // rlca
                rlc(&self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "rlca");
                4
            }
            0x08 => {
                // ld (nn), sp
                addr = self.fetch16(mem);
                mem.write16(addr, self.sp);
                // if (DEBUG) sprintf(instruction_str, "ld ($%02hx), sp", addr);
                20
            }
            0x09 => {
                // add hl, bc
                let result = self.hl() + self.bc();
                let result12 = (self.hl() & 0xfff) + (self.bc() & 0xfff);
                // z flag not affected
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, result12 > 0xfff);
                self.set_flag(FLAG_C, result > 0xffff);
                self.set_hl(result);
                // if (DEBUG) sprintf(instruction_str, "add hl, bc");
                8
            }
            0x0a => {
                // ld a, (bc)
                self.reg.a = mem.read8(self.bc());
                // if (DEBUG) sprintf(instruction_str, "ld a, (bc)");
                8
            }
            0x0b => {
                // dec bc
                self.reg.c -= 1;
                if self.reg.c == 0xff {
                    self.reg.b -= 1;
                }
                // if (DEBUG) sprintf(instruction_str, "dec bc");
                8
            }
            0x0c => {
                // inc c
                inc8(&self.reg.c);
                // if (DEBUG) sprintf(instruction_str, "inc c");
                4
            }
            0x0d => {
                // dec c
                dec8(&self.reg.c);
                // if (DEBUG) sprintf(instruction_str, "dec c");
                4
            }
            0x0e => {
                // ld c, n
                self.reg.c = self.fetch8(mem);
                // if (DEBUG) sprintf(instruction_str, "ld c, $%02x", self.reg.c);
                8
            }
            0x0f => {
                // rrca
                rrc(&self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "rrca");
                4
            }
            0x10 => {
                // stop
                // TODO implement correctly
                self.pc -= 1;
                //self.halt = 1;
                // if (DEBUG) sprintf(instruction_str, "stop");
                4
            }
            0x11 => {
                // ld de, nn
                let val = self.fetch16(mem);
                self.set_de(val);
                // if (DEBUG) sprintf(instruction_str, "ld de, $%04x", immediate16);
                12
            }
            0x12 => {
                // ld (de), a
                mem.write8(self.de(), self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "ld (de), a");
                8
            }
            0x13 => {
                // inc de
                self.reg.e += 1;
                if self.reg.e == 0x00 {
                    self.reg.d += 1;
                }
                // if (DEBUG) sprintf(instruction_str, "inc de");
                8
            }
            0x14 => {
                // inc d
                inc8(&self.reg.d);
                // if (DEBUG) sprintf(instruction_str, "inc d");
                4
            }
            0x15 => {
                // dec d
                dec8(&self.reg.d);
                // if (DEBUG) sprintf(instruction_str, "dec d");
                4
            }
            0x16 => {
                // ld d, n
                self.reg.d = self.fetch8(mem);
                // if (DEBUG) sprintf(instruction_str, "ld d, $%02x", self.reg.d);
                8
            }
            0x17 => {
                // rla
                rl(&self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "rla");
                4
            }
            0x18 => {
                // jr n
                let offset = self.fetch8(mem) as i8; // signed value
                self.pc += (offset as i16) as u16;
                // if (DEBUG) sprintf(instruction_str, "jr $%02hhx", offset);
                8
            }
            0x19 => {
                // add hl, de
                let result = self.hl() + self.de();
                let result12 = (self.hl() & 0xfff) + (self.de() & 0xfff);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, result12 & BIT11 != 0);
                self.set_flag(FLAG_C, result & BIT15 != 0);
                self.set_hl(result);
                // if (DEBUG) sprintf(instruction_str, "add hl, de");
                8
            }
            0x1a => {
                // ld a, (de)
                ld_rm(&self.reg.a, self.de());
                // if (DEBUG) sprintf(instruction_str, "ld a, (de)");
                8
            }
            0x1b => {
                // dec de
                self.reg.e -= 1;
                if self.reg.e == 0xff {
                    self.reg.d -= 1;
                }
                // if (DEBUG) sprintf(instruction_str, "dec de");
                8
            }
            0x1c => {
                // inc e
                inc8(&self.reg.e);
                // if (DEBUG) sprintf(instruction_str, "inc e");
                4
            }
            0x1d => {
                // dec e
                dec8(&self.reg.e);
                // if (DEBUG) sprintf(instruction_str, "dec e");
                4
            }
            0x1e => {
                // ld e, n
                self.reg.e = self.fetch8(mem);
                // if (DEBUG) sprintf(instruction_str, "ld e, $%02x", self.reg.e);
                8
            }
            0x1f => {
                // rra
                rr(&self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "rra");
                4
            }
            0x20 => {
                // jr nz, n
                // let offset = self.fetch8(mem) as i8; // signed value
                // if self.nz() { self.pc += (int16_t) offset; }
                // if (DEBUG) sprintf(instruction_str, "jr nz, $%02hhx", offset);
                8
            }
            0x21 => {
                // ld hl, nn
                let val = self.fetch16(mem);
                self.set_hl(val);
                // if (DEBUG) sprintf(instruction_str, "ld hl, $%04x", self.hl());
                12
            }
            0x22 => {
                // ldi (hl), a
                mem.write8(self.hl(), self.reg.a);
                self.reg.l += 1;
                if self.reg.l == 0x00 {
                    self.reg.h += 1;
                }
                // if (DEBUG) sprintf(instruction_str, "ldi (hl), a");
                8
            }
            0x23 => {
                // inc hl
                self.reg.l += 1;
                if self.reg.l == 0x00 {
                    self.reg.h += 1;
                }
                // if (DEBUG) sprintf(instruction_str, "inc hl");
                8
            }
            0x24 => {
                // inc h
                inc8(&self.reg.h);
                // if (DEBUG) sprintf(instruction_str, "inc h");
                4
            }
            0x25 => {
                // dec h
                dec8(&self.reg.h);
                // if (DEBUG) sprintf(instruction_str, "dec h");
                4
            }
            0x26 => {
                // ld h, n
                self.reg.h = self.fetch8(mem);
                // if (DEBUG) sprintf(instruction_str, "ld h, $%02x", self.reg.h);
                8
            }
            0x27 => {
                // daa
                daa();
                // if (DEBUG) sprintf(instruction_str, "daa");
                4
            }
            0x28 => {
                // jr z, n
                // let offset = self.fetch8(mem) as i8; // signed value
                // if self.z() { self.pc += (int16_t) offset; }
                // if (DEBUG) sprintf(instruction_str, "jr z, $%02hhx", offset);
                8
            }
            0x29 => {
                // add hl, hl
                let result = self.hl() + self.hl();
                let result12 = (self.hl() & 0xfff) + (self.hl() & 0xfff);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, result12 & BIT11 != 0);
                self.set_flag(FLAG_C, result & BIT15 != 0);
                self.set_hl(result);
                // if (DEBUG) sprintf(instruction_str, "add hl, hl");
                8
            }
            0x2a => {
                // ldi a, (hl)
                self.reg.a = mem.read8(self.hl());
                self.reg.l += 1;
                if self.reg.l == 0x00 {
                    self.reg.h += 1;
                }
                // if (DEBUG) sprintf(instruction_str, "ldi a, (hl)");
                8
            }
            0x2b => {
                // dec hl
                self.reg.l -= 1;
                if self.reg.l == 0xff {
                    self.reg.h -= 1;
                }
                // if (DEBUG) sprintf(instruction_str, "dec hl");
                8
            }
            0x2c => {
                // inc l
                inc8(&self.reg.l);
                // if (DEBUG) sprintf(instruction_str, "inc l");
                4
            }
            0x2d => {
                // dec l
                dec8(&self.reg.l);
                // if (DEBUG) sprintf(instruction_str, "dec l");
                4
            }
            0x2e => {
                // ld l, n
                byte = self.fetch8(mem);
                self.reg.l = byte;
                // if (DEBUG) sprintf(instruction_str, "ld l, $%02hhx", byte);
                8
            }
            0x2f => {
                // cpl a
                self.reg.a = !self.reg.a;
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, true);
                // if (DEBUG) sprintf(instruction_str, "cpl a");
                4
            }
            0x30 => {
                // jr nc, n
                // offset = self.fetch8(mem) as i8; // signed value
                // if self.nc() { self.pc += (int16_t) offset; }
                // if (DEBUG) sprintf(instruction_str, "jr nc, $%02hhx", offset);
                8
            }
            0x31 => {
                // ld sp, nn
                self.sp = self.fetch16(mem);
                // if (DEBUG) sprintf(instruction_str, "ld sp, $%04x", self.sp);
                12
            }
            0x32 => {
                // ldd (hl), a
                mem.write8(self.hl(), self.reg.a);
                self.set_hl(self.hl() - 1);
                // if (DEBUG) sprintf(instruction_str, "ldd (hl), a");
                8
            }
            0x33 => {
                // inc sp
                self.sp += 1;
                // if (DEBUG) sprintf(instruction_str, "inc sp");
                8
            }
            0x34 => {
                // inc (hl)
                result = mem.read8(self.hl()) + 1;
                mem.write8(self.hl(), result & 0xff);
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, result > 0xff);
                // if (DEBUG) sprintf(instruction_str, "inc (hl)");
                12
            }
            0x35 => {
                // dec (hl)
                byte = mem.read8(self.hl());
                result = byte - 1;
                result4 = (byte & 0xf) - 1;
                mem.write8(self.hl(), result & 0xff);
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, result4 < 0);
                // if (DEBUG) sprintf(instruction_str, "dec (hl)");
                12
            }
            0x36 => {
                // ld (hl), n
                byte = self.fetch8(mem);
                mem.write8(self.hl(), byte);
                // if (DEBUG) sprintf(instruction_str, "ld (hl), $%02x", byte);
                12
            }
            0x37 => {
                // scf
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, true);
                // if (DEBUG) sprintf(instruction_str, "scf");
                4
            }
            0x38 => {
                // jr c, n
                // offset = self.fetch8(mem) as i8; // signed value
                // if self.c() { self.pc += (int16_t) offset; }
                // if (DEBUG) sprintf(instruction_str, "jr c, $%02hhx", offset);
                8
            }
            0x39 => {
                // add hl, sp
                let result = self.hl() + self.sp;
                let result4 = (self.hl() & 0xfff) + (self.sp & 0xfff);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, result4 & BIT11 == 0);
                self.set_flag(FLAG_C, result & BIT15 == 0);
                self.set_hl(result);
                // if (DEBUG) sprintf(instruction_str, "add hl, sp");
                8
            }
            0x3a => {
                // ldd a, (hl)
                self.reg.a = mem.read8(self.hl());
                self.reg.l -= 1;
                if self.reg.l == 0xff {
                    self.reg.h -= 1;
                }
                // if (DEBUG) sprintf(instruction_str, "ldd a, (hl)");
                8
            }
            0x3b => {
                // dec sp
                self.sp -= 1;
                // if (DEBUG) sprintf(instruction_str, "dec sp");
                8
            }
            0x3c => {
                // inc a
                result = self.reg.a + 1;
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, result > 0xff);
                self.reg.a = result;
                // if (DEBUG) sprintf(instruction_str, "inc a");
                4
            }
            0x3d => {
                // dec a
                result = self.reg.a - 1;
                result4 = (self.reg.a & 0xf) - 1;
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, result < 0);
                self.reg.a = result & 0xff;
                // if (DEBUG) sprintf(instruction_str, "dec a");
                4
            }
            0x3e => {
                // ld a, n
                self.reg.a = self.fetch8(mem);
                // if (DEBUG) sprintf(instruction_str, "ld a, $%02x", self.reg.a);
                16
            }
            0x3f => {
                // ccf
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, !self.c());
                // if (DEBUG) sprintf(instruction_str, "ccf");
                4
            }
            0x40 => {
                // ld b, b
                self.reg.b = self.reg.b;
                // if (DEBUG) sprintf(instruction_str, "ld b, b");
                4
            }
            0x41 => {
                // ld b, c
                self.reg.b = self.reg.c;
                // if (DEBUG) sprintf(instruction_str, "ld b, c");
                4
            }
            0x42 => {
                // ld b, d
                self.reg.b = self.reg.d;
                // if (DEBUG) sprintf(instruction_str, "ld b, d");
                4
            }
            0x43 => {
                // ld b, e
                self.reg.b = self.reg.e;
                // if (DEBUG) sprintf(instruction_str, "ld b, e");
                4
            }
            0x44 => {
                // ld b, h
                self.reg.b = self.reg.h;
                // if (DEBUG) sprintf(instruction_str, "ld b, h");
                4
            }
            0x45 => {
                // ld b, l
                self.reg.b = self.reg.l;
                // if (DEBUG) sprintf(instruction_str, "ld b, l");
                4
            }
            0x46 => {
                // ld b, (hl)
                self.reg.b = mem.read8(self.hl());
                // if (DEBUG) sprintf(instruction_str, "ld b, (hl)");
                8
            }
            0x47 => {
                // ld b, a
                self.reg.b = self.reg.a;
                // if (DEBUG) sprintf(instruction_str, "ld b, a");
                4
            }
            0x48 => {
                // ld c, b
                self.reg.c = self.reg.b;
                // if (DEBUG) sprintf(instruction_str, "ld c, b");
                4
            }
            0x49 => {
                // ld c, c
                self.reg.c = self.reg.c;
                // if (DEBUG) sprintf(instruction_str, "ld c, c");
                4
            }
            0x4a => {
                // ld c, d
                self.reg.c = self.reg.d;
                // if (DEBUG) sprintf(instruction_str, "ld c, d");
                4
            }
            0x4b => {
                // ld c, e
                ld_rr(&self.reg.c, &self.reg.e);
                // if (DEBUG) sprintf(instruction_str, "ld c, e");
                4
            }
            0x4c => {
                // ld c, h
                ld_rr(&self.reg.c, &self.reg.h);
                // if (DEBUG) sprintf(instruction_str, "ld c, h");
                4
            }
            0x4d => {
                // ld c, l
                ld_rr(&self.reg.c, &self.reg.l);
                // if (DEBUG) sprintf(instruction_str, "ld c, l");
                4
            }
            0x4e => {
                // ld c, (hl)
                self.reg.c = mem.read8(self.hl());
                // if (DEBUG) sprintf(instruction_str, "ld c, (hl)");
                8
            }
            0x4f => {
                // ld c, a
                self.reg.c = self.reg.a;
                // if (DEBUG) sprintf(instruction_str, "ld c, a");
                4
            }
            0x50 => {
                // ld d, b
                self.reg.d = self.reg.b;
                // if (DEBUG) sprintf(instruction_str, "ld d, b");
                4
            }
            0x51 => {
                // ld d, c
                self.reg.d = self.reg.c;
                // if (DEBUG) sprintf(instruction_str, "ld d, c");
                4
            }
            0x52 => {
                // ld d, d
                self.reg.d = self.reg.d;
                // if (DEBUG) sprintf(instruction_str, "ld d, d");
                4
            }
            0x53 => {
                // ld d, e
                self.reg.d = self.reg.e;
                // if (DEBUG) sprintf(instruction_str, "ld d, e");
                4
            }
            0x54 => {
                // ld d, h
                self.reg.d = self.reg.h;
                // if (DEBUG) sprintf(instruction_str, "ld d, h");
                4
            }
            0x55 => {
                // ld d, l
                self.reg.d = self.reg.l;
                // if (DEBUG) sprintf(instruction_str, "ld d, l");
                4
            }
            0x56 => {
                // ld d, (hl)
                self.reg.d = mem.read8(self.hl());
                // if (DEBUG) sprintf(instruction_str, "ld d, (hl)");
                8
            }
            0x57 => {
                // ld d, a
                self.reg.d = self.reg.a;
                // if (DEBUG) sprintf(instruction_str, "ld d, a");
                4
            }
            0x58 => {
                // ld e, b
                self.reg.e = self.reg.b;
                // if (DEBUG) sprintf(instruction_str, "ld e, b");
                4
            }
            0x59 => {
                // ld e, c
                self.reg.e = self.reg.c;
                // if (DEBUG) sprintf(instruction_str, "ld e, c");
                4
            }
            0x5a => {
                // ld e, d
                self.reg.e = self.reg.d;
                // if (DEBUG) sprintf(instruction_str, "ld e, d");
                4
            }
            0x5b => {
                // ld e, e
                self.reg.e = self.reg.e;
                // if (DEBUG) sprintf(instruction_str, "ld e, e");
                4
            }
            0x5c => {
                // ld e, h
                self.reg.e = self.reg.h;
                // if (DEBUG) sprintf(instruction_str, "ld e, h");
                4
            }
            0x5d => {
                // ld e, l
                self.reg.e = self.reg.l;
                // if (DEBUG) sprintf(instruction_str, "ld e, l");
                4
            }
            0x5e => {
                // ld e, (hl)
                self.reg.e = mem.read8(self.hl());
                // if (DEBUG) sprintf(instruction_str, "ld e, (hl)");
                8
            }
            0x5f => {
                // ld e, a
                self.reg.e = self.reg.a;
                // if (DEBUG) sprintf(instruction_str, "ld e, a");
                4
            }
            0x60 => {
                // ld h, b
                self.reg.h = self.reg.b;
                // if (DEBUG) sprintf(instruction_str, "ld h, b");
                4
            }
            0x61 => {
                // ld h, c
                self.reg.h = self.reg.c;
                // if (DEBUG) sprintf(instruction_str, "ld h, c");
                4
            }
            0x62 => {
                // ld h, d
                self.reg.h = self.reg.d;
                // if (DEBUG) sprintf(instruction_str, "ld h, d");
                4
            }
            0x63 => {
                // ld h, e
                self.reg.h = self.reg.e;
                // if (DEBUG) sprintf(instruction_str, "ld h, e");
                4
            }
            0x64 => {
                // ld h, h
                self.reg.h = self.reg.h;
                // if (DEBUG) sprintf(instruction_str, "ld h, h");
                4
            }
            0x65 => {
                // ld h, l
                self.reg.h = self.reg.l;
                // if (DEBUG) sprintf(instruction_str, "ld h, l");
                4
            }
            0x66 => {
                // ld h, (hl)
                self.reg.h = mem.read8(self.hl());
                // if (DEBUG) sprintf(instruction_str, "ld h, (hl)");
                8
            }
            0x67 => {
                // ld h, a
                self.reg.h = self.reg.a;
                // if (DEBUG) sprintf(instruction_str, "ld h, a");
                4
            }
            0x68 => {
                // ld l, b
                self.reg.l = self.reg.b;
                // if (DEBUG) sprintf(instruction_str, "ld l, b");
                4
            }
            0x69 => {
                // ld l, c
                self.reg.l = self.reg.c;
                // if (DEBUG) sprintf(instruction_str, "ld l, c");
                4
            }
            0x6a => {
                // ld l, d
                self.reg.l = self.reg.d;
                // if (DEBUG) sprintf(instruction_str, "ld l, d");
                4
            }
            0x6b => {
                // ld l, e
                self.reg.l = self.reg.e;
                // if (DEBUG) sprintf(instruction_str, "ld l, e");
                4
            }
            0x6c => {
                // ld l, h
                self.reg.l = self.reg.h;
                // if (DEBUG) sprintf(instruction_str, "ld l, h");
                4
            }
            0x6d => {
                // ld l, l
                self.reg.l = self.reg.l;
                // if (DEBUG) sprintf(instruction_str, "ld l, l");
                4
            }
            0x6e => {
                // ld l, (hl)
                self.reg.l = mem.read8(self.hl());
                // if (DEBUG) sprintf(instruction_str, "ld l, (hl)");
                8
            }
            0x6f => {
                // ld l, a
                self.reg.l = self.reg.a;
                // if (DEBUG) sprintf(instruction_str, "ld l, a");
                4
            }
            0x70 => {
                // ld (hl), b
                mem.write8(self.hl(), self.reg.b);
                // if (DEBUG) sprintf(instruction_str, "ld (hl), b");
                8
            }
            0x71 => {
                // ld (hl), c
                mem.write8(self.hl(), self.reg.c);
                // if (DEBUG) sprintf(instruction_str, "ld (hl), c");
                8
            }
            0x72 => {
                // ld (hl), d
                mem.write8(self.hl(), self.reg.d);
                // if (DEBUG) sprintf(instruction_str, "ld (hl), d");
                8
            }
            0x73 => {
                // ld (hl), e
                mem.write8(self.hl(), self.reg.e);
                // if (DEBUG) sprintf(instruction_str, "ld (hl), e");
                8
            }
            0x74 => {
                // ld (hl), h
                mem.write8(self.hl(), self.reg.h);
                // if (DEBUG) sprintf(instruction_str, "ld (hl), h");
                8
            }
            0x75 => {
                // ld (hl), l
                mem.write8(self.hl(), self.reg.l);
                // if (DEBUG) sprintf(instruction_str, "ld (hl), l");
                8
            }
            0x76 => {
                // halt
                //self.pc -= 1;
                //printf("halting!\n");
                //exit(1);
                // if (DEBUG) sprintf(instruction_str, "halt");
                4
            }
            0x77 => {
                // ld (hl), a
                mem.write8(self.hl(), self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "ld (hl), a");
                8
            }
            0x78 => {
                // ld a, b
                self.reg.a = self.reg.b;
                // if (DEBUG) sprintf(instruction_str, "ld a, b");
                4
            }
            0x79 => {
                // ld a, c
                self.reg.a = self.reg.c;
                // if (DEBUG) sprintf(instruction_str, "ld a, c");
                4
            }
            0x7a => {
                // ld a, d
                self.reg.a = self.reg.d;
                // if (DEBUG) sprintf(instruction_str, "ld a, d");
                4
            }
            0x7b => {
                // ld a, e
                self.reg.a = self.reg.e;
                // if (DEBUG) sprintf(instruction_str, "ld a, e");
                4
            }
            0x7c => {
                // ld a, h
                self.reg.a = self.reg.h;
                // if (DEBUG) sprintf(instruction_str, "ld a, h");
                4
            }
            0x7d => {
                // ld a, l
                self.reg.a = self.reg.l;
                // if (DEBUG) sprintf(instruction_str, "ld a, l");
                4
            }
            0x7e => {
                // ld a, (hl)
                self.reg.a = mem.read8(self.hl());
                // if (DEBUG) sprintf(instruction_str, "ld a, (hl)");
                8
            }
            0x7f => {
                // ld a, a
                self.reg.a = self.reg.a;
                // if (DEBUG) sprintf(instruction_str, "ld a, a");
                4
            }
            0x80 => {
                // add a, b
                add(&self.reg.b);
                // if (DEBUG) sprintf(instruction_str, "add a, b");
                4
            }
            0x81 => {
                // add a, c
                add(&self.reg.c);
                // if (DEBUG) sprintf(instruction_str, "add a, c");
                4
            }
            0x82 => {
                // add a, d
                add(&self.reg.d);
                // if (DEBUG) sprintf(instruction_str, "add a, d");
                4
            }
            0x83 => {
                // add a, e
                add(&self.reg.e);
                // if (DEBUG) sprintf(instruction_str, "add a, e");
                4
            }
            0x84 => {
                // add a, h
                add(&self.reg.h);
                // if (DEBUG) sprintf(instruction_str, "add a, h");
                4
            }
            0x85 => {
                // add a, l
                add(&self.reg.l);
                // if (DEBUG) sprintf(instruction_str, "add a, l");
                4
            }
            0x86 => {
                // add a, (hl)
                byte = mem.read8(self.hl());
                result = self.reg.a + byte;
                result4 = (self.reg.a & 0xf) * (byte & 0xf);
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, result4 > 0xf);
                self.set_flag(FLAG_C, result > 0xff);
                self.reg.a = result;
                // if (DEBUG) sprintf(instruction_str, "add a, (hl)");
                8
            }
            0x87 => {
                // add a, a
                add(&self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "add a, a");
                4
            }
            0x88 => {
                // adc a, b
                adc(&self.reg.b);
                // if (DEBUG) sprintf(instruction_str, "adc a, b");
                4
            }
            0x89 => {
                // adc a, c
                adc(&self.reg.c);
                // if (DEBUG) sprintf(instruction_str, "adc a, c");
                4
            }
            0x8a => {
                adc(&self.reg.d);
                // if (DEBUG) sprintf(instruction_str, "adc a, d");
                4
            }
            0x8b => {
                adc(&self.reg.e);
                // if (DEBUG) sprintf(instruction_str, "adc a, e");
                4
            }
            0x8c => {
                adc(&self.reg.h);
                // if (DEBUG) sprintf(instruction_str, "adc a, h");
                4
            }
            0x8d => {
                adc(&self.reg.l);
                // if (DEBUG) sprintf(instruction_str, "adc a, l");
                4
            }
            0x8e => {
                // adc a, (hl)
                byte = mem.read8(self.hl());
                if self.c() {
                    byte += 1;
                }
                result = self.reg.a + byte;
                result4 = (self.reg.a & 0xf) + (byte & 0xf);
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, result4 > 0xf);
                self.set_flag(FLAG_C, result > 0xff);
                self.reg.a = result;
                // if (DEBUG) sprintf(instruction_str, "add a, (hl)");
                8
            }
            0x8f => {
                adc(&self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "adc a, a");
                4
            }
            0x90 => {
                // sub a, b
                sub(&self.reg.b);
                // if (DEBUG) sprintf(instruction_str, "sub a, b");
                4
            }
            0x91 => {
                // sub a, c
                sub(&self.reg.c);
                // if (DEBUG) sprintf(instruction_str, "sub a, c");
                4
            }
            0x92 => {
                // sub a, d
                sub(&self.reg.d);
                // if (DEBUG) sprintf(instruction_str, "sub a, d");
                4
            }
            0x93 => {
                // sub a, e
                sub(&self.reg.e);
                // if (DEBUG) sprintf(instruction_str, "sub a, e");
                4
            }
            0x94 => {
                // sub a, h
                sub(&self.reg.h);
                // if (DEBUG) sprintf(instruction_str, "sub a, h");
                8
            }
            0x95 => {
                // sub a, l
                sub(&self.reg.l);
                // if (DEBUG) sprintf(instruction_str, "sub a, l");
                8
            }
            0x96 => {
                // sub a, (hl)
                byte = mem.read8(self.hl());
                sub(&byte);
                // if (DEBUG) sprintf(instruction_str, "sub a, (hl)");
                8
            }
            0x97 => {
                // sub a, a
                sub(&self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "sub a, a");
                4
            }
            0x98 => {
                // sbc a, b
                sbc(&self.reg.b);
                // if (DEBUG) sprintf(instruction_str, "sbc a, b");
                4
            }
            0x99 => {
                // sbc a, c
                sbc(&self.reg.c);
                // if (DEBUG) sprintf(instruction_str, "sbc a, c");
                4
            }
            0x9a => {
                // sbc a, d
                sbc(&self.reg.d);
                // if (DEBUG) sprintf(instruction_str, "sbc a, d");
                4
            }
            0x9b => {
                // sbc a, e
                sbc(&self.reg.e);
                // if (DEBUG) sprintf(instruction_str, "sbc a, e");
                4
            }
            0x9c => {
                // sbc a, h
                sbc(&self.reg.h);
                // if (DEBUG) sprintf(instruction_str, "sbc a, h");
                4
            }
            0x9d => {
                // sbc a, l
                sbc(&self.reg.l);
                // if (DEBUG) sprintf(instruction_str, "sbc a, l");
                4
            }
            0x9e => {
                // sbc a, (hl)
                byte = mem.read8(self.hl());
                sbc(&byte);
                // if (DEBUG) sprintf(instruction_str, "sbc a, (hl)");
                8
            }
            0x9f => {
                // sbc a, a
                sbc(&self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "sbc a, a");
                4
            }
            0xa0 => {
                // and a, b
                and(&self.reg.b);
                // if (DEBUG) sprintf(instruction_str, "and a, b");
                4
            }
            0xa1 => {
                // and a, c
                and(&self.reg.c);
                // if (DEBUG) sprintf(instruction_str, "and a, c");
                4
            }
            0xa2 => {
                // and a, d
                and(&self.reg.d);
                // if (DEBUG) sprintf(instruction_str, "and a, d");
                4
            }
            0xa3 => {
                // and a, e
                and(&self.reg.e);
                // if (DEBUG) sprintf(instruction_str, "and a, e");
                4
            }
            0xa4 => {
                // and a, h
                and(&self.reg.h);
                // if (DEBUG) sprintf(instruction_str, "and a, h");
                4
            }
            0xa5 => {
                // and a, l
                and(&self.reg.l);
                // if (DEBUG) sprintf(instruction_str, "and a, l");
                4
            }
            0xa6 => {
                // and a, (hl)
                and(&mem.data[self.hl() as usize]);
                // if (DEBUG) sprintf(instruction_str, "and a, (hl)");
                8
            }
            0xa7 => {
                // and a, a
                and(&self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "and a, a");
                4
            }
            0xa8 => {
                // xor a, b
                xor(&self.reg.b);
                // if (DEBUG) sprintf(instruction_str, "xor a, b");
                4
            }
            0xa9 => {
                // xor a, c
                xor(&self.reg.c);
                // if (DEBUG) sprintf(instruction_str, "xor a, c");
                4
            }
            0xaa => {
                // xor a, d
                xor(&self.reg.d);
                // if (DEBUG) sprintf(instruction_str, "xor a, d");
                4
            }
            0xab => {
                // xor a, e
                xor(&self.reg.e);
                // if (DEBUG) sprintf(instruction_str, "xor a, e");
                4
            }
            0xac => {
                // xor a, h
                xor(&self.reg.h);
                // if (DEBUG) sprintf(instruction_str, "xor a, h");
                4
            }
            0xad => {
                // xor a, l
                xor(&self.reg.l);
                // if (DEBUG) sprintf(instruction_str, "xor a, l");
                4
            }
            0xae => {
                // xor a, (hl)
                xor(&mem.data[self.hl() as usize]);
                // if (DEBUG) sprintf(instruction_str, "xor a, (hl)");
                8
            }
            0xaf => {
                // xor a, a
                xor(&self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "xor a, a");
                4
            }
            0xb0 => {
                // or b
                or(&self.reg.b);
                // if (DEBUG) sprintf(instruction_str, "or b");
                4
            }
            0xb1 => {
                // or c
                or(&self.reg.c);
                // if (DEBUG) sprintf(instruction_str, "or c");
                4
            }
            0xb2 => {
                // or d
                or(&self.reg.d);
                // if (DEBUG) sprintf(instruction_str, "or d");
                4
            }
            0xb3 => {
                // or e
                or(&self.reg.e);
                // if (DEBUG) sprintf(instruction_str, "or e");
                4
            }
            0xb4 => {
                // or h
                or(&self.reg.h);
                // if (DEBUG) sprintf(instruction_str, "or h");
                4
            }
            0xb5 => {
                // or l
                or(&self.reg.l);
                // if (DEBUG) sprintf(instruction_str, "or l");
                4
            }
            0xb6 => {
                // or (hl)
                byte = mem.data[self.hl() as usize];
                or(&byte);
                // if (DEBUG) sprintf(instruction_str, "or (hl)");
                4
            }
            0xb7 => {
                // or a
                or(&self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "or a");
                4
            }
            0xb8 => {
                // cp a, b
                cp(&self.reg.b);
                // if (DEBUG) sprintf(instruction_str, "cp a, b");
                4
            }
            0xb9 => {
                // cp a, c
                cp(&self.reg.c);
                // if (DEBUG) sprintf(instruction_str, "cp a, c");
                4
            }
            0xba => {
                // cp a, d
                cp(&self.reg.d);
                // if (DEBUG) sprintf(instruction_str, "cp a, d");
                4
            }
            0xbb => {
                // cp a, e
                cp(&self.reg.e);
                // if (DEBUG) sprintf(instruction_str, "cp a, e");
                4
            }
            0xbc => {
                // cp a, h
                cp(&self.reg.h);
                // if (DEBUG) sprintf(instruction_str, "cp a, h");
                4
            }
            0xbd => {
                // cp a, l
                cp(&self.reg.l);
                // if (DEBUG) sprintf(instruction_str, "cp a, l");
                4
            }
            0xbe => {
                // cp a, (hl)
                byte = mem.read8(self.hl());
                cp(&byte);
                // if (DEBUG) sprintf(instruction_str, "cp a, (hl)");
                8
            }
            0xbf => {
                // cp a, a
                cp(&self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "cp a, a");
                4
            }
            0xc0 => {
                // ret nz
                if self.nz() {
                    self.pc = self.pop_stack(mem);
                }
                // if (DEBUG) sprintf(instruction_str, "ret nz");
                8
            }
            0xc1 => {
                // pop bc
                let val = self.pop_stack(mem);
                self.set_bc(val);
                // if (DEBUG) sprintf(instruction_str, "pop bc");
                12
            }
            0xc2 => {
                // jp nz, nn
                addr = self.fetch16(mem);
                if self.nz() {
                    self.pc = addr;
                }
                // if (DEBUG) sprintf(instruction_str, "jp nz, $%04x", addr);
                12
            }
            0xc3 => {
                // jp nn
                self.pc = self.fetch16(mem);
                // if (DEBUG) sprintf(instruction_str, "jp $%04x", self.pc);
                12
            }
            0xc4 => {
                // call nz, nn
                addr = self.fetch16(mem);
                if self.nz() {
                    self.push_stack(mem, self.pc);
                    self.pc = addr;
                }
                // if (DEBUG) sprintf(instruction_str, "call nz, $%04x", addr);
                12
            }
            0xc5 => {
                // push bc
                self.push_stack(mem, self.bc());
                // if (DEBUG) sprintf(instruction_str, "push bc");
                16
            }
            0xc6 => {
                // add a, #
                byte = self.fetch8(mem);
                add(&byte);
                // if (DEBUG) sprintf(instruction_str, "add a, $%02x", immediate);
                8
            }
            0xc7 => {
                // rst $00
                rst(0x00);
                // if (DEBUG) sprintf(instruction_str, "rst $00");
                32
            }
            0xc8 => {
                // ret z
                if self.z() {
                    self.pc = self.pop_stack(mem);
                }
                // if (DEBUG) sprintf(instruction_str, "ret z");
                8
            }
            0xc9 => {
                // ret
                self.pc = self.pop_stack(mem);
                // if (DEBUG) sprintf(instruction_str, "ret");
                8
            }
            0xca => {
                // jp z, nn
                addr = self.fetch16(mem);
                if self.z() {
                    self.pc = addr;
                }
                // if (DEBUG) sprintf(instruction_str, "jp z, $%04x", addr);
                12
            }
            0xcc => {
                // call z, nn
                let addr = self.fetch16(mem);
                if self.z() {
                    self.push_stack(mem, self.pc);
                    self.pc = addr;
                }
                // if (DEBUG) sprintf(instruction_str, "call z, $%04x", addr);
                12
            }
            0xcd => {
                // call nn
                addr = self.fetch16(mem);
                self.push_stack(mem, self.pc);
                self.pc = addr;
                // if (DEBUG) sprintf(instruction_str, "call $%04x", addr);
                12
            }
            0xce => {
                // adc a, #
                byte = self.fetch8(mem);
                adc(&byte);
                // if (DEBUG) sprintf(instruction_str, "adc a, $%02hhx", byte);
                8
            }
            0xcf => {
                // rst $8
                self.push_stack(mem, self.pc);
                self.pc = 0x8;
                // if (DEBUG) sprintf(instruction_str, "rst $8");
                32
            }
            0xd0 => {
                // ret nc
                if self.nc() {
                    self.pc = self.pop_stack(mem);
                }
                // if (DEBUG) sprintf(instruction_str, "ret nc");
                8
            }
            0xd1 => {
                // pop de
                let val = self.pop_stack(mem);
                self.set_de(val);
                // if (DEBUG) sprintf(instruction_str, "pop de");
                12
            }
            0xd2 => {
                // jp nc, nn
                addr = self.fetch16(mem);
                if self.nc() {
                    self.pc = addr;
                }
                // if (DEBUG) sprintf(instruction_str, "jp nc, $%04x", addr);
                12
            }
            0xd3 => {
                // invalid instruction
                // if (DEBUG) sprintf(instruction_str, "invalid instruction d3");
                0
            }
            0xd4 => {
                // call nc, nn
                addr = self.fetch16(mem);
                if self.nc() {
                    self.push_stack(mem, self.pc);
                    self.pc = addr;
                }
                // if (DEBUG) sprintf(instruction_str, "call nc, $%04x", addr);
                12
            }
            0xd5 => {
                // push de
                self.push_stack(mem, self.de());
                // if (DEBUG) sprintf(instruction_str, "push de");
                16
            }
            0xd6 => {
                // sub a, #
                byte = self.fetch8(mem);
                sub(&byte);
                // if (DEBUG) sprintf(instruction_str, "sub a, $%02x", immediate);
                8
            }
            0xd7 => {
                // rst $10
                rst(0x10);
                // if (DEBUG) sprintf(instruction_str, "rst $10");
                32
            }
            0xd8 => {
                // ret c
                if self.c() {
                    self.pc = self.pop_stack(mem);
                }
                // if (DEBUG) sprintf(instruction_str, "ret c");
                8
            }
            0xd9 => {
                // reti
                self.pc = self.pop_stack(mem);
                self.interrupts = true;
                // if (DEBUG) sprintf(instruction_str, "reti");
                8
            }
            0xda => {
                // jp c, nn
                addr = self.fetch16(mem);
                if self.c() {
                    self.pc = addr;
                }
                // if (DEBUG) sprintf(instruction_str, "jp c, $%04x", addr);
                12
            }
            0xdb => {
                // invalid instruction
                // if (DEBUG) sprintf(instruction_str, "invalid instruction db");
                0
            }
            0xdc => {
                // call c, nn
                addr = self.fetch16(mem);
                if self.c() {
                    self.push_stack(mem, self.pc);
                    self.pc = addr;
                }
                // if (DEBUG) sprintf(instruction_str, "call c, $%04x", addr);
                12
            }
            0xdd => {
                // invalid instruction
                // if (DEBUG) sprintf(instruction_str, "invalid instruction dd");
                0
            }
            0xde => {
                // sbc a, #
                byte = self.fetch8(mem);
                sbc(&byte);
                // if (DEBUG) sprintf(instruction_str, "sbc a, $%02x", byte);
                4
            }
            0xdf => {
                // rst $18
                self.push_stack(mem, self.pc);
                self.pc = 0x18;
                // if (DEBUG) sprintf(instruction_str, "rst $18");
                32
            }
            0xe0 => {
                // ld ($ff00+n), a
                let n = self.fetch8(mem) as u16;
                mem.write8(0xff00 + n, self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "ld ($ff00+$%02hhx), a", byte);
                12
            }
            0xe1 => {
                // pop hl
                let val = self.pop_stack(mem);
                self.set_hl(val);
                // if (DEBUG) sprintf(instruction_str, "pop hl");
                12
            }
            0xe2 => {
                // ld ($ff00+c), a
                mem.write8(0xff00 + self.reg.c as u16, self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "ld ($ff00+c), a");
                8
            }
            0xe3 => {
                // invalid instruction
                // if (DEBUG) sprintf(instruction_str, "invalid instruction e3");
                0
            }
            0xe4 => {
                // invalid instruction
                // if (DEBUG) sprintf(instruction_str, "invalid instruction e4");
                0
            }
            0xe5 => {
                // push hl
                self.push_stack(mem, self.hl());
                // if (DEBUG) sprintf(instruction_str, "push hl");
                16
            }
            0xe6 => {
                // and a, #
                byte = self.fetch8(mem);
                result = self.reg.a & byte;
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, true);
                self.set_flag(FLAG_C, false);
                self.reg.a = result;
                // if (DEBUG) sprintf(instruction_str, "and a, $%02x", byte);
                8
            }
            0xe7 => {
                // rst $20
                self.push_stack(mem, self.pc);
                self.pc = 0x20;
                // if (DEBUG) sprintf(instruction_str, "rst $20");
                32
            }
            0xe8 => {
                // add sp, #
                // let immediate = self.fetch8(mem) as i8 as i16; // signed
                // let result = self.sp + immediate;
                // let result4 = (self.sp & 0x0f) + (immediate & 0x0f);
                // self.set_flag(FLAG_Z, false);
                // self.set_flag(FLAG_N, false);
                // if immediate < 0 {
                // 	self.set_flag(FLAG_H, result4 > (immediate & 0xf));
                // 	self.set_flag(FLAG_C, result > immediate);
                // } else {
                // 	self.set_flag(FLAG_H, result4 > 0xf);
                // 	self.set_flag(FLAG_C, result > 0xff);
                // }
                // self.sp += immediate;
                // if (DEBUG) sprintf(instruction_str, "add sp, $%02x", immediate);
                16
            }
            0xe9 => {
                // jp hl
                // i've seen this instruction as jp (hl) but the parenthesis seem misleading
                // since we aren't dereferencing hl, leaving them out
                self.pc = self.hl();
                // if (DEBUG) sprintf(instruction_str, "jp hl");
                4
            }
            0xea => {
                // ld (nn), a
                addr = self.fetch16(mem);
                mem.write8(addr, self.reg.a);
                // if (DEBUG) sprintf(instruction_str, "ld ($%04x), a", addr);
                16
            }
            0xeb => {
                // invalid instruction
                // if (DEBUG) sprintf(instruction_str, "invalid instruction eb");
                0
            }
            0xec => {
                // invalid instruction
                // if (DEBUG) sprintf(instruction_str, "invalid instruction ec");
                0
            }
            0xed => {
                // invalid instruction
                // if (DEBUG) sprintf(instruction_str, "invalid instruction ed");
                0
            }
            0xee => {
                // xor #
                byte = self.fetch8(mem);
                xor(&byte);
                // if (DEBUG) sprintf(instruction_str, "xor $%02x", byte);
                8
            }
            0xef => {
                // rst $28
                rst(0x28);
                // if (DEBUG) sprintf(instruction_str, "rst $28");
                32
            }
            0xf0 => {
                // ld a, ($ff00+n)
                let val = self.fetch8(mem) as u16;
                self.reg.a = mem.read8(0xff00 + val);
                // if (DEBUG) sprintf(instruction_str, "ld a, ($ff00+$%02hhx), %04x", byte, 0xff00 + byte);
                12
            }
            0xf1 => {
                // pop af
                let val = self.pop_stack(mem);
                self.set_af(val);
                // if (DEBUG) sprintf(instruction_str, "pop af");
                12
            }
            0xf2 => {
                // invalid instruction
                // if (DEBUG) sprintf(instruction_str, "invalid instruction f2");
                0
            }
            0xf3 => {
                // disable int
                self.interrupts = false;
                // if (DEBUG) sprintf(instruction_str, "di");
                4
            }
            0xf4 => {
                // invalid instruction
                // if (DEBUG) sprintf(instruction_str, "invalid instruction f4");
                0
            }
            0xf5 => {
                // push af
                let val = self.af();
                self.push_stack(mem, val);
                // if (DEBUG) sprintf(instruction_str, "push af");
                16
            }
            0xf6 => {
                // or a, #
                byte = self.fetch8(mem);
                self.reg.a |= byte;
                self.set_flag(FLAG_Z, self.reg.a == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, false);
                // if (DEBUG) sprintf(instruction_str, "or a, $%02x", byte);
                8
            }
            0xf7 => {
                // rst $30
                self.push_stack(mem, self.pc);
                self.pc = 0x30;
                // if (DEBUG) sprintf(instruction_str, "rst $30");
                32
            }
            0xf8 => {
                // ld hl, sp+$n
                // let val = self.fetch8(mem) as u16;
                // let result = (self.sp & 0xff) + val;
                // // let result4 = (self.sp & 0xf) + (immediate & 0xf);
                // self.set_flag(FLAG_Z, false);
                // self.set_flag(FLAG_N, false);
                // self.set_flag(FLAG_H, result4 > 0xf);
                // self.set_flag(FLAG_C, result > 0xff);
                // //printf("ld hl, sp+%d, sp=%02hx\n", immediate, self.sp);
                // self.set_hl(self.sp + immediate);
                // if (DEBUG) sprintf(instruction_str, "ld hl, sp%c$%02x", immediate < 0 ? '-' : '+', immediate);
                12
            }
            0xf9 => {
                // ld sp, hl
                self.sp = self.hl();
                // if (DEBUG) sprintf(instruction_str, "ld sp, hl");
                8
            }
            0xfa => {
                // ld a, (nn)
                addr = self.fetch16(mem);
                self.reg.a = mem.read8(addr);
                // if (DEBUG) sprintf(instruction_str, "ld a, ($%04x)", addr);
                16
            }
            0xfb => {
                // enable int
                self.interrupts = true;
                // if (DEBUG) sprintf(instruction_str, "ei");
                4
            }
            0xfc => {
                // invalid instruction
                // if (DEBUG) sprintf(instruction_str, "invalid instruction fc");
                0
            }
            0xfd => {
                // invalid instruction
                // if (DEBUG) sprintf(instruction_str, "invalid instruction fd");
                0
            }
            0xfe => {
                // cp a, n
                byte = self.fetch8(mem);
                cp(&byte);
                // if (DEBUG) sprintf(instruction_str, "cp a, $%02hhx", byte);
                8
            }
            0xff => {
                // rst $38
                self.push_stack(mem, self.pc);
                self.pc = 0x38;
                // if (DEBUG) sprintf(instruction_str, "rst $38");
                32
            }
            _ => {
                eprintln!("Unhandled instruction: 0x{:02X}", op);
                std::process::exit(1);
            }
        }
    }

    fn execute_cb(&self, op: u8) -> u16 {
        match op {
            _ => {
                eprintln!("Unhandled instruction: 0xCB 0x{:02X}", op);
                std::process::exit(1);
            }
        }
    }
}

fn add(val: &u8) -> u8 {
    0
}

fn sub(val: &u8) -> u8 {
    0
}

fn cp(val: &u8) -> u8 {
    0
}

fn xor(val: &u8) -> u8 {
    0
}

fn or(val: &u8) -> u8 {
    0
}

fn and(val: &u8) -> u8 {
    0
}

fn adc(val: &u8) -> u8 {
    0
}

fn sbc(val: &u8) -> u8 {
    0
}

fn inc8(val: &u8) -> u8 {
    0
}

fn dec8(val: &u8) -> u8 {
    0
}

fn rlc(val: &u8) -> u8 {
    0
}

fn rr(val: &u8) -> u8 {
    0
}

fn rrc(val: &u8) -> u8 {
    0
}

fn rl(val: &u8) -> u8 {
    0
}

fn daa() {}

fn ld_rr(val: &u8, b: &u8) -> u8 {
    0
}

fn ld_rm(val: &u8, b: u16) -> u8 {
    0
}

fn rst(val: u8) {}
