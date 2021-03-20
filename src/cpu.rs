use crate::memory::Memory;

macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);

        // Find and cut the rest of the path
        match &name[..name.len() - 3].rfind(':') {
            Some(pos) => &name[pos + 1..name.len() - 3],
            None => &name[..name.len() - 3],
        }
    }};
}

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
    pub f: u8,
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            pc: 0x0100,
            sp: 0xfffe,
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
        self.reg.f = val as u8;
        self.reg.a = (val >> 8) as u8;
    }

    pub fn bc(&self) -> u16 {
        ((self.reg.b as u16) << 8) | (self.reg.c as u16)
    }

    pub fn set_bc(&mut self, val: u16) {
        self.reg.c = val as u8;
        self.reg.b = (val >> 8) as u8;
    }

    pub fn de(&self) -> u16 {
        ((self.reg.d as u16) << 8) | (self.reg.e as u16)
    }

    pub fn set_de(&mut self, val: u16) {
        self.reg.e = val as u8;
        self.reg.d = (val >> 8) as u8;
    }

    pub fn hl(&self) -> u16 {
        ((self.reg.h as u16) << 8) | (self.reg.l as u16)
    }

    pub fn set_hl(&mut self, val: u16) {
        self.reg.l = val as u8;
        self.reg.h = (val >> 8) as u8;
    }

    // Flags

    pub fn set_flag(&mut self, flag: u8, val: bool) {
        if val {
            self.reg.f |= flag;
        } else {
            self.reg.f &= !flag;
        }
    }

    pub fn nz(&self) -> bool {
        (self.reg.f & FLAG_Z) == 0
    }

    pub fn z(&self) -> bool {
        self.reg.f & FLAG_Z != 0
    }

    pub fn nc(&self) -> bool {
        self.reg.f & FLAG_C == 0
    }

    pub fn c(&self) -> bool {
        self.reg.f & FLAG_C != 0
    }

    pub fn set_interrupt(&mut self, mem: &mut Memory, interrupt: Interrupt) {
        mem.data[0xff0f] |= interrupt as u8;
    }

    pub fn unset_interrupt(&mut self, mem: &mut Memory, interrupt: Interrupt) {
        mem.data[0xff0f] &= !(interrupt as u8);
    }

    pub fn interrupt_reg(&self, mem: &Memory) -> u8 {
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
        self.pc += 2;
        result
    }

    // Execute

    pub fn step(&mut self, mem: &mut Memory) -> u16 {
        let op = self.fetch8(mem);
        eprintln!("Found 0x{:02X} at PC=0x{:04x}", op, self.pc - 1);

        // DEBUG
        if self.pc > 0x8000 && self.pc < 0xff00 {
            // panic!("We probably shouldn't be here...");
        }

        let cycles: u16;

        if op == 0xcb {
            let op2 = self.fetch8(mem);
            cycles = self.execute_cb(mem, op2);
        } else {
            cycles = self.execute(mem, op);
        }

        if self.interrupts {
            let interrupt_flag = &mem.data[0xff0f];
            let interrupt_enable = self.interrupt_reg(&mem);
            if interrupt_enable & Interrupt::VBlank as u8 & *interrupt_flag != 0 {
                self.interrupts = false;
                self.push_stack(mem, self.pc);
                self.pc = 0x0040;
                self.unset_interrupt(mem, Interrupt::VBlank);
            } else if interrupt_enable & Interrupt::LCDC as u8 & *interrupt_flag != 0 {
                self.interrupts = false;
                self.push_stack(mem, self.pc);
                self.pc = 0x0048;
                self.unset_interrupt(mem, Interrupt::LCDC);
            } else if interrupt_enable & Interrupt::Timer as u8 & *interrupt_flag != 0 {
                self.interrupts = false;
                self.push_stack(mem, self.pc);
                self.pc = 0x0050;
                self.unset_interrupt(mem, Interrupt::Timer);
            } else if interrupt_enable & Interrupt::Serial as u8 & *interrupt_flag != 0 {
                self.interrupts = false;
                self.push_stack(mem, self.pc);
                self.pc = 0x0058;
                self.unset_interrupt(mem, Interrupt::Serial);
            } else if interrupt_enable & Interrupt::JoyPad as u8 & *interrupt_flag != 0 {
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
            *mem.reg_div() = mem.reg_div().wrapping_add(1);
        }
    }

    fn execute(&mut self, mem: &mut Memory, op: u8) -> u16 {
        let mut byte: u8;
        let immediate: u16;
        let result4: u8;
        let result: u8;
        let addr: u16;

        match op {
            0x00 => 0,
            0x01 => {
                // ld bc, nn
                immediate = self.fetch16(mem);
                self.set_bc(immediate);
                12
            }
            0x02 => {
                // ld (bc), a
                mem.write8(self.bc(), self.reg.a);
                8
            }
            0x03 => {
                // inc bc
                self.reg.c = self.reg.c.wrapping_add(1);
                if self.reg.c == 0x00 {
                    self.reg.b = self.reg.b.wrapping_add(1);
                }
                8
            }
            0x04 => {
                // inc b
                inc8(&mut self.reg.b, &mut self.reg.f, false)
            }
            0x05 => {
                // dec b
                dec8(&mut self.reg.b, &mut self.reg.f, false)
            }
            0x06 => {
                // ld b, n
                self.reg.b = self.fetch8(mem);
                8
            }
            0x07 => {
                // rlca
                rlc(&mut self.reg.a, &mut self.reg.f, false)
            }
            0x08 => {
                // ld (nn), sp
                addr = self.fetch16(mem);
                mem.write16(addr, self.sp);
                20
            }
            0x09 => {
                // add hl, bc
                let (result, overflow) = self.hl().overflowing_add(self.bc());
                let result12 = (self.hl() & 0xfff) + (self.bc() & 0xfff);
                // z flag not affected
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, result12 > 0xfff);
                self.set_flag(FLAG_C, overflow);
                self.set_hl(result);
                8
            }
            0x0a => {
                // ld a, (bc)
                self.reg.a = mem.read8(self.bc());
                8
            }
            0x0b => {
                // dec bc
                self.reg.c = self.reg.c.wrapping_sub(1);
                if self.reg.c == 0xff {
                    self.reg.b = self.reg.b.wrapping_sub(1);
                }
                8
            }
            0x0c => {
                // inc c
                inc8(&mut self.reg.c, &mut self.reg.f, false)
            }
            0x0d => {
                // dec c
                dec8(&mut self.reg.c, &mut self.reg.f, false)
            }
            0x0e => {
                // ld c, n
                self.reg.c = self.fetch8(mem);
                8
            }
            0x0f => {
                // rrca
                rrc(&mut self.reg.a, &mut self.reg.f, false);
                4
            }
            0x10 => {
                // stop
                // TODO implement correctly
                panic!("STOP");
                // self.pc -= 1;
                //self.halt = 1;
                // 4
            }
            0x11 => {
                // ld de, nn
                let val = self.fetch16(mem);
                self.set_de(val);
                12
            }
            0x12 => {
                // ld (de), a
                mem.write8(self.de(), self.reg.a);
                8
            }
            0x13 => {
                // inc de
                self.reg.e = self.reg.e.wrapping_add(1);
                if self.reg.e == 0x00 {
                    self.reg.d = self.reg.d.wrapping_add(1);
                }
                8
            }
            0x14 => {
                // inc d
                inc8(&mut self.reg.d, &mut self.reg.f, false)
            }
            0x15 => {
                // dec d
                dec8(&mut self.reg.d, &mut self.reg.f, false)
            }
            0x16 => {
                // ld d, n
                self.reg.d = self.fetch8(mem);
                8
            }
            0x17 => {
                // rla
                rl(&mut self.reg.a, &mut self.reg.f, false)
            }
            0x18 => {
                // jr n
                let offset = self.fetch8(mem) as i8; // signed value
                self.pc = self.pc.wrapping_add(offset as u16);
                8
            }
            0x19 => {
                // add hl, de
                let result = self.hl().wrapping_add(self.de());
                let result12 = (self.hl() & 0xfff) + (self.de() & 0xfff);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, result12 & BIT11 != 0);
                self.set_flag(FLAG_C, result & BIT15 != 0);
                self.set_hl(result);
                8
            }
            0x1a => {
                // ld a, (de)
                let de = self.de();
                ld_rm(&mut self.reg.a, de, &mem)
            }
            0x1b => {
                // dec de
                self.reg.e -= 1;
                if self.reg.e == 0xff {
                    self.reg.d -= 1;
                }
                8
            }
            0x1c => {
                // inc e
                inc8(&mut self.reg.e, &mut self.reg.f, false)
            }
            0x1d => {
                // dec e
                dec8(&mut self.reg.e, &mut self.reg.f, false)
            }
            0x1e => {
                // ld e, n
                self.reg.e = self.fetch8(mem);
                8
            }
            0x1f => {
                // rra
                rr(&mut self.reg.a, &mut self.reg.f, false);
                4
            }
            0x20 => {
                // jr nz, n
                let offset = self.fetch8(mem) as i8; // signed value
                if self.nz() {
                    self.pc = self.pc.wrapping_add(offset as u16);
                }
                8
            }
            0x21 => {
                // ld hl, nn
                let val = self.fetch16(mem);
                self.set_hl(val);
                12
            }
            0x22 => {
                // ldi (hl), a
                mem.write8(self.hl(), self.reg.a);
                self.reg.l = self.reg.l.wrapping_add(1);
                if self.reg.l == 0x00 {
                    self.reg.h = self.reg.h.wrapping_add(1);
                }
                8
            }
            0x23 => {
                // inc hl
                self.reg.l = self.reg.l.wrapping_add(1);
                if self.reg.l == 0x00 {
                    self.reg.h = self.reg.h.wrapping_add(1);
                }
                8
            }
            0x24 => {
                // inc h
                inc8(&mut self.reg.h, &mut self.reg.f, false)
            }
            0x25 => {
                // dec h
                dec8(&mut self.reg.h, &mut self.reg.f, false)
            }
            0x26 => {
                // ld h, n
                self.reg.h = self.fetch8(mem);
                8
            }
            0x27 => {
                // daa
                daa(&mut self.reg.a, &mut self.reg.f)
            }
            0x28 => {
                // jr z, n
                let offset = self.fetch8(mem) as i8; // signed value
                if self.z() {
                    self.pc = self.pc.wrapping_add(offset as u16);
                }
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
                8
            }
            0x2a => {
                // ldi a, (hl)
                self.reg.a = mem.read8(self.hl());
                self.reg.l = self.reg.l.wrapping_add(1);
                if self.reg.l == 0x00 {
                    self.reg.h = self.reg.h.wrapping_add(1);
                }
                8
            }
            0x2b => {
                // dec hl
                self.reg.l -= 1;
                if self.reg.l == 0xff {
                    self.reg.h -= 1;
                }
                8
            }
            0x2c => {
                // inc l
                inc8(&mut self.reg.l, &mut self.reg.f, false)
            }
            0x2d => {
                // dec l
                dec8(&mut self.reg.l, &mut self.reg.f, false)
            }
            0x2e => {
                // ld l, n
                byte = self.fetch8(mem);
                self.reg.l = byte;
                8
            }
            0x2f => {
                // cpl a
                self.reg.a = !self.reg.a;
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, true);
                4
            }
            0x30 => {
                // jr nc, n
                let offset = self.fetch8(mem) as i8; // signed value
                if self.nc() {
                    self.pc = self.pc.wrapping_add(offset as u16);
                }
                8
            }
            0x31 => {
                // ld sp, nn
                self.sp = self.fetch16(mem);
                12
            }
            0x32 => {
                // ldd (hl), a
                mem.write8(self.hl(), self.reg.a);
                self.reg.l = self.reg.l.wrapping_sub(1);
                if self.reg.l == 0xff {
                    self.reg.h = self.reg.h.wrapping_sub(1);
                }
                8
            }
            0x33 => {
                // inc sp
                self.sp += 1;
                8
            }
            0x34 => {
                // inc (hl)
                let (result, overflow) = mem.read8(self.hl()).overflowing_add(1);
                mem.write8(self.hl(), result & 0xff);
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, overflow);
                12
            }
            0x35 => {
                // dec (hl)
                byte = mem.read8(self.hl());
                result = byte.wrapping_sub(1);
                mem.write8(self.hl(), result);
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, byte & 0xf != 0);
                12
            }
            0x36 => {
                // ld (hl), n
                byte = self.fetch8(mem);
                mem.write8(self.hl(), byte);
                12
            }
            0x37 => {
                // scf
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, true);
                4
            }
            0x38 => {
                // jr c, n
                let offset = self.fetch8(mem) as i8; // signed value
                if self.c() {
                    self.pc = self.pc.wrapping_add(offset as u16);
                }
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
                8
            }
            0x3a => {
                // ldd a, (hl)
                self.reg.a = mem.read8(self.hl());
                self.reg.l -= 1;
                if self.reg.l == 0xff {
                    self.reg.h -= 1;
                }
                8
            }
            0x3b => {
                // dec sp
                self.sp -= 1;
                8
            }
            0x3c => {
                // inc a
                inc8(&mut self.reg.a, &mut self.reg.f, false)
            }
            0x3d => {
                // dec a
                dec8(&mut self.reg.a, &mut self.reg.f, false)
            }
            0x3e => {
                // ld a, n
                self.reg.a = self.fetch8(mem);
                16
            }
            0x3f => {
                // ccf
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, !self.c());
                4
            }
            0x40 => {
                // ld b, b
                self.reg.b = self.reg.b;
                4
            }
            0x41 => {
                // ld b, c
                self.reg.b = self.reg.c;
                4
            }
            0x42 => {
                // ld b, d
                self.reg.b = self.reg.d;
                4
            }
            0x43 => {
                // ld b, e
                self.reg.b = self.reg.e;
                4
            }
            0x44 => {
                // ld b, h
                self.reg.b = self.reg.h;
                4
            }
            0x45 => {
                // ld b, l
                self.reg.b = self.reg.l;
                4
            }
            0x46 => {
                // ld b, (hl)
                self.reg.b = mem.read8(self.hl());
                8
            }
            0x47 => {
                // ld b, a
                self.reg.b = self.reg.a;
                4
            }
            0x48 => {
                // ld c, b
                self.reg.c = self.reg.b;
                4
            }
            0x49 => {
                // ld c, c
                self.reg.c = self.reg.c;
                4
            }
            0x4a => {
                // ld c, d
                self.reg.c = self.reg.d;
                4
            }
            0x4b => {
                // ld c, e
                ld_rr(&mut self.reg.c, &self.reg.e);
                4
            }
            0x4c => {
                // ld c, h
                ld_rr(&mut self.reg.c, &self.reg.h);
                4
            }
            0x4d => {
                // ld c, l
                ld_rr(&mut self.reg.c, &self.reg.l);
                4
            }
            0x4e => {
                // ld c, (hl)
                self.reg.c = mem.read8(self.hl());
                8
            }
            0x4f => {
                // ld c, a
                self.reg.c = self.reg.a;
                4
            }
            0x50 => {
                // ld d, b
                self.reg.d = self.reg.b;
                4
            }
            0x51 => {
                // ld d, c
                self.reg.d = self.reg.c;
                4
            }
            0x52 => {
                // ld d, d
                self.reg.d = self.reg.d;
                4
            }
            0x53 => {
                // ld d, e
                self.reg.d = self.reg.e;
                4
            }
            0x54 => {
                // ld d, h
                self.reg.d = self.reg.h;
                4
            }
            0x55 => {
                // ld d, l
                self.reg.d = self.reg.l;
                4
            }
            0x56 => {
                // ld d, (hl)
                self.reg.d = mem.read8(self.hl());
                8
            }
            0x57 => {
                // ld d, a
                self.reg.d = self.reg.a;
                4
            }
            0x58 => {
                // ld e, b
                self.reg.e = self.reg.b;
                4
            }
            0x59 => {
                // ld e, c
                self.reg.e = self.reg.c;
                4
            }
            0x5a => {
                // ld e, d
                self.reg.e = self.reg.d;
                4
            }
            0x5b => {
                // ld e, e
                self.reg.e = self.reg.e;
                4
            }
            0x5c => {
                // ld e, h
                self.reg.e = self.reg.h;
                4
            }
            0x5d => {
                // ld e, l
                self.reg.e = self.reg.l;
                4
            }
            0x5e => {
                // ld e, (hl)
                self.reg.e = mem.read8(self.hl());
                8
            }
            0x5f => {
                // ld e, a
                self.reg.e = self.reg.a;
                4
            }
            0x60 => {
                // ld h, b
                self.reg.h = self.reg.b;
                4
            }
            0x61 => {
                // ld h, c
                self.reg.h = self.reg.c;
                4
            }
            0x62 => {
                // ld h, d
                self.reg.h = self.reg.d;
                4
            }
            0x63 => {
                // ld h, e
                self.reg.h = self.reg.e;
                4
            }
            0x64 => {
                // ld h, h
                self.reg.h = self.reg.h;
                4
            }
            0x65 => {
                // ld h, l
                self.reg.h = self.reg.l;
                4
            }
            0x66 => {
                // ld h, (hl)
                self.reg.h = mem.read8(self.hl());
                8
            }
            0x67 => {
                // ld h, a
                self.reg.h = self.reg.a;
                4
            }
            0x68 => {
                // ld l, b
                self.reg.l = self.reg.b;
                4
            }
            0x69 => {
                // ld l, c
                self.reg.l = self.reg.c;
                4
            }
            0x6a => {
                // ld l, d
                self.reg.l = self.reg.d;
                4
            }
            0x6b => {
                // ld l, e
                self.reg.l = self.reg.e;
                4
            }
            0x6c => {
                // ld l, h
                self.reg.l = self.reg.h;
                4
            }
            0x6d => {
                // ld l, l
                self.reg.l = self.reg.l;
                4
            }
            0x6e => {
                // ld l, (hl)
                self.reg.l = mem.read8(self.hl());
                8
            }
            0x6f => {
                // ld l, a
                self.reg.l = self.reg.a;
                4
            }
            0x70 => {
                // ld (hl), b
                mem.write8(self.hl(), self.reg.b);
                8
            }
            0x71 => {
                // ld (hl), c
                mem.write8(self.hl(), self.reg.c);
                8
            }
            0x72 => {
                // ld (hl), d
                mem.write8(self.hl(), self.reg.d);
                8
            }
            0x73 => {
                // ld (hl), e
                mem.write8(self.hl(), self.reg.e);
                8
            }
            0x74 => {
                // ld (hl), h
                mem.write8(self.hl(), self.reg.h);
                8
            }
            0x75 => {
                // ld (hl), l
                mem.write8(self.hl(), self.reg.l);
                8
            }
            0x76 => {
                // halt
                panic!("HALT");
                // 4
            }
            0x77 => {
                // ld (hl), a
                mem.write8(self.hl(), self.reg.a);
                8
            }
            0x78 => {
                // ld a, b
                self.reg.a = self.reg.b;
                4
            }
            0x79 => {
                // ld a, c
                self.reg.a = self.reg.c;
                4
            }
            0x7a => {
                // ld a, d
                self.reg.a = self.reg.d;
                4
            }
            0x7b => {
                // ld a, e
                self.reg.a = self.reg.e;
                4
            }
            0x7c => {
                // ld a, h
                self.reg.a = self.reg.h;
                4
            }
            0x7d => {
                // ld a, l
                self.reg.a = self.reg.l;
                4
            }
            0x7e => {
                // ld a, (hl)
                self.reg.a = mem.read8(self.hl());
                8
            }
            0x7f => {
                // ld a, a
                self.reg.a = self.reg.a;
                4
            }
            0x80 => {
                // add a, b
                add(&mut self.reg.a, self.reg.b, &mut self.reg.f, false)
            }
            0x81 => {
                // add a, c
                add(&mut self.reg.a, self.reg.c, &mut self.reg.f, false)
            }
            0x82 => {
                // add a, d
                add(&mut self.reg.a, self.reg.d, &mut self.reg.f, false)
            }
            0x83 => {
                // add a, e
                add(&mut self.reg.a, self.reg.e, &mut self.reg.f, false)
            }
            0x84 => {
                // add a, h
                add(&mut self.reg.a, self.reg.h, &mut self.reg.f, false)
            }
            0x85 => {
                // add a, l
                add(&mut self.reg.a, self.reg.l, &mut self.reg.f, false)
            }
            0x86 => {
                // add a, (hl)
                byte = mem.read8(self.hl());
                let (result, overflow) = self.reg.a.overflowing_add(byte);
                result4 = (self.reg.a & 0xf) + (byte & 0xf);
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, result4 > 0xf);
                self.set_flag(FLAG_C, overflow);
                self.reg.a = result;
                8
            }
            0x87 => {
                // add a, a
                let a = self.reg.a;
                add(&mut self.reg.a, a, &mut self.reg.f, false)
            }
            0x88 => {
                // adc a, b
                adc(&mut self.reg.a, self.reg.b, &mut self.reg.f, false)
            }
            0x89 => {
                // adc a, c
                adc(&mut self.reg.a, self.reg.c, &mut self.reg.f, false)
            }
            0x8a => {
                adc(&mut self.reg.a, self.reg.d, &mut self.reg.f, false)
            }
            0x8b => {
                adc(&mut self.reg.a, self.reg.e, &mut self.reg.f, false)
            }
            0x8c => {
                adc(&mut self.reg.a, self.reg.h, &mut self.reg.f, false)
            }
            0x8d => {
                adc(&mut self.reg.a, self.reg.l, &mut self.reg.f, false)
            }
            0x8e => {
                // adc a, (hl)
                byte = mem.read8(self.hl());
                if self.c() {
                    byte += 1;
                }
                let (result, overflow) = self.reg.a.overflowing_add(byte);
                result4 = (self.reg.a & 0xf) + (byte & 0xf);
                self.set_flag(FLAG_Z, result == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, result4 > 0xf);
                self.set_flag(FLAG_C, overflow);
                self.reg.a = result;
                8
            }
            0x8f => {
                let a = self.reg.a;
                adc(&mut self.reg.a, a, &mut self.reg.f, false)
            }
            0x90 => {
                // sub a, b
                sub(&mut self.reg.a, self.reg.b, &mut self.reg.f, false)
            }
            0x91 => {
                // sub a, c
                sub(&mut self.reg.a, self.reg.c, &mut self.reg.f, false)
            }
            0x92 => {
                // sub a, d
                sub(&mut self.reg.a, self.reg.d, &mut self.reg.f, false)
            }
            0x93 => {
                // sub a, e
                sub(&mut self.reg.a, self.reg.e, &mut self.reg.f, false)
            }
            0x94 => {
                // sub a, h
                sub(&mut self.reg.a, self.reg.h, &mut self.reg.f, false)
            }
            0x95 => {
                // sub a, l
                sub(&mut self.reg.a, self.reg.l, &mut self.reg.f, false)
            }
            0x96 => {
                // sub a, (hl)
                byte = mem.read8(self.hl());
                sub(&mut self.reg.a, byte, &mut self.reg.f, true)
            }
            0x97 => {
                // sub a, a
                let a = self.reg.a;
                sub(&mut self.reg.a, a, &mut self.reg.f, false)
            }
            0x98 => {
                // sbc a, b
                sbc(&mut self.reg.a, self.reg.b, &mut self.reg.f, false)
            }
            0x99 => {
                // sbc a, c
                sbc(&mut self.reg.a, self.reg.c, &mut self.reg.f, false)
            }
            0x9a => {
                // sbc a, d
                sbc(&mut self.reg.a, self.reg.d, &mut self.reg.f, false)
            }
            0x9b => {
                // sbc a, e
                sbc(&mut self.reg.a, self.reg.e, &mut self.reg.f, false)
            }
            0x9c => {
                // sbc a, h
                sbc(&mut self.reg.a, self.reg.h, &mut self.reg.f, false)
            }
            0x9d => {
                // sbc a, l
                sbc(&mut self.reg.a, self.reg.l, &mut self.reg.f, false)
            }
            0x9e => {
                // sbc a, (hl)
                byte = mem.read8(self.hl());
                sbc(&mut self.reg.a, byte, &mut self.reg.f, false)
            }
            0x9f => {
                // sbc a, a
                let a = self.reg.a;
                sbc(&mut self.reg.a, a, &mut self.reg.f, false)
            }
            0xa0 => {
                // and a, b
                and(&mut self.reg.a, self.reg.b, &mut self.reg.f, false)
            }
            0xa1 => {
                // and a, c
                and(&mut self.reg.a, self.reg.c, &mut self.reg.f, false)
            }
            0xa2 => {
                // and a, d
                and(&mut self.reg.a, self.reg.d, &mut self.reg.f, false)
            }
            0xa3 => {
                // and a, e
                and(&mut self.reg.a, self.reg.e, &mut self.reg.f, false)
            }
            0xa4 => {
                // and a, h
                and(&mut self.reg.a, self.reg.h, &mut self.reg.f, false)
            }
            0xa5 => {
                // and a, l
                and(&mut self.reg.a, self.reg.l, &mut self.reg.f, false)
            }
            0xa6 => {
                // and a, (hl)
                let indirect = mem.read8(self.hl());
                and(&mut self.reg.a, indirect, &mut self.reg.f, false)
            }
            0xa7 => {
                // and a, a
                let a = self.reg.a;
                and(&mut self.reg.a, a, &mut self.reg.f, false);
                4
            }
            0xa8 => {
                // xor a, b
                xor(&mut self.reg.a, self.reg.b, &mut self.reg.f, false)
            }
            0xa9 => {
                // xor a, c
                xor(&mut self.reg.a, self.reg.c, &mut self.reg.f, false)
            }
            0xaa => {
                // xor a, d
                xor(&mut self.reg.a, self.reg.d, &mut self.reg.f, false)
            }
            0xab => {
                // xor a, e
                xor(&mut self.reg.a, self.reg.e, &mut self.reg.f, false)
            }
            0xac => {
                // xor a, h
                xor(&mut self.reg.a, self.reg.h, &mut self.reg.f, false)
            }
            0xad => {
                // xor a, l
                xor(&mut self.reg.a, self.reg.l, &mut self.reg.f, false)
            }
            0xae => {
                // xor a, (hl)
                let hl = mem.data[self.hl() as usize];
                xor(&mut self.reg.a, hl, &mut self.reg.f, true)
            }
            0xaf => {
                // xor a, a
                let a = self.reg.a;
                xor(&mut self.reg.a, a, &mut self.reg.f, false)
            }
            0xb0 => {
                // or b
                or(&mut self.reg.a, self.reg.b, &mut self.reg.f, false)
            }
            0xb1 => {
                // or c
                or(&mut self.reg.a, self.reg.c, &mut self.reg.f, false)
            }
            0xb2 => {
                // or d
                or(&mut self.reg.a, self.reg.d, &mut self.reg.f, false)
            }
            0xb3 => {
                // or e
                or(&mut self.reg.a, self.reg.e, &mut self.reg.f, false)
            }
            0xb4 => {
                // or h
                or(&mut self.reg.a, self.reg.h, &mut self.reg.f, false)
            }
            0xb5 => {
                // or l
                or(&mut self.reg.a, self.reg.l, &mut self.reg.f, false)
            }
            0xb6 => {
                // or (hl)
                byte = mem.data[self.hl() as usize];
                or(&mut self.reg.a, byte, &mut self.reg.f, true)
            }
            0xb7 => {
                // or a
                let a = self.reg.a;
                or(&mut self.reg.a, a, &mut self.reg.f, false)
            }
            0xb8 => {
                // cp a, b
                cp(&mut self.reg.a, self.reg.b, &mut self.reg.f, false)
            }
            0xb9 => {
                // cp a, c
                cp(&mut self.reg.a, self.reg.c, &mut self.reg.f, false)
            }
            0xba => {
                // cp a, d
                cp(&mut self.reg.a, self.reg.d, &mut self.reg.f, false)
            }
            0xbb => {
                // cp a, e
                cp(&mut self.reg.a, self.reg.e, &mut self.reg.f, false)
            }
            0xbc => {
                // cp a, h
                cp(&mut self.reg.a, self.reg.h, &mut self.reg.f, false)
            }
            0xbd => {
                // cp a, l
                cp(&mut self.reg.a, self.reg.l, &mut self.reg.f, false)
            }
            0xbe => {
                // cp a, (hl)
                byte = mem.read8(self.hl());
                cp(&mut self.reg.a, byte, &mut self.reg.f, true)
            }
            0xbf => {
                // cp a, a
                let a = self.reg.a;
                cp(&mut self.reg.a, a, &mut self.reg.f, false)
            }
            0xc0 => {
                // ret nz
                if self.nz() {
                    self.pc = self.pop_stack(mem);
                }
                8
            }
            0xc1 => {
                // pop bc
                let val = self.pop_stack(mem);
                self.set_bc(val);
                12
            }
            0xc2 => {
                // jp nz, nn
                addr = self.fetch16(mem);
                if self.nz() {
                    self.pc = addr;
                }
                12
            }
            0xc3 => {
                // jp nn
                self.pc = self.fetch16(mem);
                12
            }
            0xc4 => {
                // call nz, nn
                addr = self.fetch16(mem);
                if self.nz() {
                    self.push_stack(mem, self.pc);
                    self.pc = addr;
                }
                12
            }
            0xc5 => {
                // push bc
                self.push_stack(mem, self.bc());
                16
            }
            0xc6 => {
                // add a, #
                byte = self.fetch8(mem);
                add(&mut self.reg.a, byte, &mut self.reg.f, true)
            }
            0xc7 => {
                // rst $00
                self.push_stack(mem, self.pc);
                self.pc = 0x0;
                32
            }
            0xc8 => {
                // ret z
                if self.z() {
                    self.pc = self.pop_stack(mem);
                }
                8
            }
            0xc9 => {
                // ret
                self.pc = self.pop_stack(mem);
                8
            }
            0xca => {
                // jp z, nn
                addr = self.fetch16(mem);
                if self.z() {
                    self.pc = addr;
                }
                12
            }
            0xcc => {
                // call z, nn
                let addr = self.fetch16(mem);
                if self.z() {
                    self.push_stack(mem, self.pc);
                    self.pc = addr;
                }
                12
            }
            0xcd => {
                // call nn
                addr = self.fetch16(mem);
                self.push_stack(mem, self.pc);
                self.pc = addr;
                12

            }
            0xce => {
                // adc a, #
                byte = self.fetch8(mem);
                adc(&mut self.reg.a, byte, &mut self.reg.f, false)
            }
            0xcf => {
                // rst $8
                self.push_stack(mem, self.pc);
                self.pc = 0x8;
                32
            }
            0xd0 => {
                // ret nc
                if self.nc() {
                    self.pc = self.pop_stack(mem);
                }
                8
            }
            0xd1 => {
                // pop de
                let val = self.pop_stack(mem);
                self.set_de(val);
                12
            }
            0xd2 => {
                // jp nc, nn
                addr = self.fetch16(mem);
                if self.nc() {
                    self.pc = addr;
                }
                12
            }
            0xd3 => {
                panic!("Invalid instruction: 0x{:02X}", op);
            }
            0xd4 => {
                // call nc, nn
                addr = self.fetch16(mem);
                if self.nc() {
                    self.push_stack(mem, self.pc);
                    self.pc = addr;
                }
                12
            }
            0xd5 => {
                // push de
                self.push_stack(mem, self.de());
                16
            }
            0xd6 => {
                // sub a, #
                byte = self.fetch8(mem);
                sub(&mut self.reg.a, byte, &mut self.reg.f, false)
            }
            0xd7 => {
                // rst $10
                self.push_stack(mem, self.pc);
                self.pc = 0x10;
                32
            }
            0xd8 => {
                // ret c
                if self.c() {
                    self.pc = self.pop_stack(mem);
                }
                8
            }
            0xd9 => {
                // reti
                self.pc = self.pop_stack(mem);
                self.interrupts = true;
                8
            }
            0xda => {
                // jp c, nn
                addr = self.fetch16(mem);
                if self.c() {
                    self.pc = addr;
                }
                12
            }
            0xdb => {
                panic!("Invalid instruction: 0x{:02X}", op);
            }
            0xdc => {
                // call c, nn
                addr = self.fetch16(mem);
                if self.c() {
                    self.push_stack(mem, self.pc);
                    self.pc = addr;
                }
                12
            }
            0xdd => {
                panic!("Invalid instruction: 0x{:02X}", op);
            }
            0xde => {
                // sbc a, #
                byte = self.fetch8(mem);
                sbc(&mut self.reg.a, byte, &mut self.reg.f, false)
            }
            0xdf => {
                // rst $18
                self.push_stack(mem, self.pc);
                self.pc = 0x18;
                32
            }
            0xe0 => {
                // ld ($ff00+n), a
                let n = self.fetch8(mem) as u16;
                mem.write8(0xff00 + n, self.reg.a);
                12
            }
            0xe1 => {
                // pop hl
                let val = self.pop_stack(mem);
                self.set_hl(val);
                12
            }
            0xe2 => {
                // ld ($ff00+c), a
                mem.write8(0xff00 + self.reg.c as u16, self.reg.a);
                8
            }
            0xe3 => {
                panic!("Invalid instruction: 0x{:02X}", op);
            }
            0xe4 => {
                panic!("Invalid instruction: 0x{:02X}", op);
            }
            0xe5 => {
                // push hl
                self.push_stack(mem, self.hl());
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
                8
            }
            0xe7 => {
                // rst $20
                self.push_stack(mem, self.pc);
                self.pc = 0x20;
                32
            }
            0xe8 => {
                panic!("unimplemented");
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
            }
            0xe9 => {
                // jp hl
                // I've seen this instruction as jp (hl) but the parenthesis seem misleading
                // since we aren't dereferencing hl, leaving them out
                self.pc = self.hl();
                4
            }
            0xea => {
                // ld (nn), a
                addr = self.fetch16(mem);
                mem.write8(addr, self.reg.a);
                16
            }
            0xeb => {
                panic!("invalid");
            }
            0xec => {
                panic!("invalid");
            }
            0xed => {
                panic!("invalid");
            }
            0xee => {
                // xor #
                byte = self.fetch8(mem);
                xor(&mut self.reg.a, byte, &mut self.reg.f, false);
                8
            }
            0xef => {
                // rst $28
                self.push_stack(mem, self.pc);
                self.pc = 0x28;
                32
            }
            0xf0 => {
                // ld a, ($ff00+n)
                let val = self.fetch8(mem) as u16;
                self.reg.a = mem.read8(0xff00 + val);
                12
            }
            0xf1 => {
                // pop af
                let val = self.pop_stack(mem);
                self.set_af(val);
                12
            }
            0xf2 => {
                panic!("Invalid instruction: 0x{:02X}", op);
            }
            0xf3 => {
                // disable int
                self.interrupts = false;
                4
            }
            0xf4 => {
                panic!("Invalid instruction: 0x{:02X}", op);
            }
            0xf5 => {
                // push af
                let val = self.af();
                self.push_stack(mem, val);
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
                8
            }
            0xf7 => {
                // rst $30
                self.push_stack(mem, self.pc);
                self.pc = 0x30;
                32
            }
            0xf8 => {
                // ld hl, sp+$n
                // panic!("Unimplemented Instruction");
                let val = self.fetch8(mem) as i8;
                let result = self.sp.wrapping_add(val as u16);
                // let result4 = (self.sp & 0xf) + (val & 0xf);
                self.set_flag(FLAG_Z, false);
                self.set_flag(FLAG_N, false);
                // self.set_flag(FLAG_H, result4 > 0xf);
                self.set_flag(FLAG_C, result > 0xff);

                self.set_hl(result);
                12
            }
            0xf9 => {
                // ld sp, hl
                self.sp = self.hl();
                8
            }
            0xfa => {
                // ld a, (nn)
                addr = self.fetch16(mem);
                self.reg.a = mem.read8(addr);
                16
            }
            0xfb => {
                // enable int
                self.interrupts = true;
                4
            }
            0xfc => {
                panic!("Invalid instruction: 0x{:02X}", op);
            }
            0xfd => {
                panic!("Invalid instruction: 0x{:02X}", op);
            }
            0xfe => {
                // cp a, n
                byte = self.fetch8(mem);
                cp(&mut self.reg.a, byte, &mut self.reg.f, true)
            }
            0xff => {
                // rst $38
                self.push_stack(mem, self.pc);
                self.pc = 0x38;
                32
            }
            _ => {
                panic!("Unhandled instruction: 0x{:02X}", op);
            }
        }
    }

    fn execute_cb(&mut self, mem: &mut Memory, op: u8) -> u16 {
        /* 0b76543210 */
        /*   xxyyyzzz */
        let x = op >> 6;
        let y = (op >> 3) & 7;
        let z = op & 7;

        let reg = match z {
            0 => &mut self.reg.b,
            1 => &mut self.reg.c,
            2 => &mut self.reg.d,
            3 => &mut self.reg.e,
            4 => &mut self.reg.h,
            5 => &mut self.reg.l,
            6 => &mut mem.data[self.hl() as usize],
            7 => &mut self.reg.a,
            _ => panic!("???"),
        };

        let indirect = z == 6;

        match x {
            0 => {
                match y {
                    2 => rl(reg, &mut self.reg.f, indirect),
                    3 => rr(reg, &mut self.reg.f, indirect),
                    4 => sla(reg, &mut self.reg.f, indirect),
                    6 => swap(reg, &mut self.reg.f, indirect),
                    7 => srl(reg, &mut self.reg.f, indirect),
                    _ => panic!("Unhandled instruction: 0xCB 0x{:02X}", op),
                }
            }
            1 => bit(*reg, y, &mut self.reg.f, indirect),
            2 => res(reg, y, indirect),
            3 => set(reg, y, indirect),
            _ => panic!("Unhandled instruction: 0xCB 0x{:02X}", op),
        }
    }
}

fn add(dst: &mut u8, src: u8, flags: &mut u8, indirect: bool) -> u16 {
    let result = dst.wrapping_add(src);
    if result == 0 {
        *flags |= FLAG_Z;
    } else {
        *flags &= !FLAG_Z;
    }
    *flags &= !FLAG_N;
    *flags |= FLAG_H; // TODO fix
    *flags |= FLAG_C; // TODO fix
    *dst = result;
    if indirect { 8 } else { 4 }
}

fn sub(dst: &mut u8, src: u8, flags: &mut u8, indirect: bool) -> u16 {
    let result = dst.wrapping_sub(src);
    if result == 0 {
        *flags |= FLAG_Z;
    } else {
        *flags &= !FLAG_Z;
    }
    *flags |= FLAG_N;
    *flags |= FLAG_H; // TODO fix
    *flags |= if *dst < src { FLAG_C } else { 0 };
    *dst = result;
    if indirect { 8 } else { 4 }
}

fn cp(dst: &mut u8, src: u8, flags: &mut u8, indirect: bool) -> u16 {
    let result = dst.wrapping_sub(src);
    if result == 0 {
        *flags |= FLAG_Z;
    } else {
        *flags &= !FLAG_Z;
    }
    *flags |= FLAG_N;
    *flags |= FLAG_H; // TODO fix
    *flags |= if *dst < src { FLAG_C } else { 0 };
    if indirect { 8 } else { 4 }
}

fn xor(dst: &mut u8, src: u8, flags: &mut u8, indirect: bool) -> u16 {
    *dst ^= src;
    *flags = if *dst == 0 { FLAG_Z } else { 0 };
    if indirect { 8 } else { 4 }
}

fn or(dst: &mut u8, src: u8, flags: &mut u8, indirect: bool) -> u16 {
    *dst |= src;
    *flags = if *dst == 0 { FLAG_Z } else { 0 };
    if indirect { 8 } else { 4 }
}

fn and(dst: &mut u8, src: u8, flags: &mut u8, indirect: bool) -> u16 {
    *dst &= src;
    *flags = if *dst == 0 { FLAG_Z | FLAG_H } else { FLAG_H };
    if indirect { 8 } else { 4 }
}

fn adc(dst: &mut u8, src: u8, flags: &mut u8, indirect: bool) -> u16 {
    let mut result = dst.wrapping_add(src);
    if *flags & FLAG_C != 0 {
        result = result.wrapping_add(1);
    }

    if result == 0 {
        *flags |= FLAG_Z;
    } else {
        *flags &= !FLAG_Z;
    }

    *flags &= !FLAG_N;
    *flags |= FLAG_H; // TODO fix
    *flags |= FLAG_C; // TODO fix
    *dst = result;
    if indirect { 8 } else { 4 }
}

fn sbc(dst: &mut u8, src: u8, flags: &mut u8, indirect: bool) -> u16 {
    let mut result = dst.wrapping_sub(src);
    if *flags & FLAG_C != 0 {
        result = result.wrapping_sub(1);
    }

    if result == 0 {
        *flags |= FLAG_Z;
    } else {
        *flags &= !FLAG_Z;
    }

    *flags |= FLAG_N;
    *flags |= FLAG_H; // TODO fix
    *flags |= FLAG_C; // TODO fix
    *dst = result;
    if indirect { 8 } else { 4 }
}

fn inc8(dst: &mut u8, flags: &mut u8, indirect: bool) -> u16 {
    *dst = dst.wrapping_add(1);
    if *dst == 0 {
        *flags |= FLAG_Z;
    } else {
        *flags &= !FLAG_Z;
    }
    *flags &= !FLAG_N;
    *flags |= FLAG_H; // TODO fix
    if indirect { 12 } else { 4 }
}

fn dec8(dst: &mut u8, flags: &mut u8, indirect: bool) -> u16 {
    *dst = dst.wrapping_sub(1);
    if *dst == 0 {
        *flags |= FLAG_Z;
    } else {
        *flags &= !FLAG_Z;
    }
    *flags |= FLAG_N;
    *flags |= FLAG_H; // TODO fix
    if indirect { 12 } else { 4 }
}

fn rlc(dst: &mut u8, flags: &mut u8, indirect: bool) -> u16 {
    let old_bit_7 = *dst & (1 << 7);
    *dst = *dst << 1;
    if *dst == 0 {
        *flags |= FLAG_Z;
    } else {
        *flags &= !FLAG_Z;
    }
    *flags &= !FLAG_N;
    *flags &= !FLAG_H;
    if old_bit_7 == 0 {
        *flags |= FLAG_C;
    } else {
        *flags &= !FLAG_C;
    }
    if indirect { 16 } else { 8 }
}

fn rr(dst: &mut u8, flags: &mut u8, indirect: bool) -> u16 {
    let bit0 = *dst & 1;
    *dst = *dst >> 1;
    if (*flags & FLAG_C) != 0 {
        *dst |= 1 << 7;
    }

    *flags = if *dst == 0 { FLAG_Z } else { 0 };
    *flags &= !FLAG_N;
    *flags &= !FLAG_H;
    *flags = if bit0 == 0 { 0 } else { FLAG_C };

    if indirect { 16 } else { 8 }
}

fn rrc(_dst: &mut u8, _flags: &mut u8, _indirect: bool) -> u16 {
    panic!("Unhandled function: {}", function!());
}

fn rl(dst: &mut u8, flags: &mut u8, indirect: bool) -> u16 {
    let bit7 = *dst & (1 << 7);
    *dst = *dst << 1;
    if (*flags & FLAG_C) != 0 {
        *dst |= 1;
    }

    *flags = if *dst == 0 { FLAG_Z } else { 0 };
    *flags &= !FLAG_N;
    *flags &= !FLAG_H;
    *flags = if bit7 == 0 { 0 } else { FLAG_C };

    if indirect { 16 } else { 8 }
}

fn srl(dst: &mut u8, flags: &mut u8, indirect: bool) -> u16 {
    let bit0 = *dst & 1;
    *dst = *dst >> 1;

    *flags = if *dst == 0 { FLAG_Z } else { 0 };
    *flags &= !FLAG_N;
    *flags &= !FLAG_H;
    *flags = if bit0 == 0 { 0 } else { FLAG_C };

    if indirect { 16 } else { 8 }
}


fn daa(dst: &mut u8, flags: &mut u8) -> u16 {
    *dst = *dst - 6 * (*dst >> 4);

    if *dst == 0 {
        *flags |= FLAG_Z;
    } else {
        *flags &= !FLAG_Z;
    }
    *flags &= !FLAG_N;
    *flags |= FLAG_C; // TODO
    4
}

fn ld_rr(dst: &mut u8, src: &u8) -> u16 {
    *dst = *src;
    4
}

fn ld_rm(dst: &mut u8, src_address: u16, mem: &Memory) -> u16 {
	*dst = mem.read8(src_address);
    8
}

fn bit(val: u8, b: u8, flags: &mut u8, indirect: bool) -> u16 {
    let result = (val >> b) & 1;
    if result == 0 {
        *flags |= FLAG_Z;
    } else {
        *flags &= !FLAG_Z;
    }
    *flags &= !FLAG_N;
    *flags |= FLAG_H;
    if indirect { 16 } else { 8 }
}

fn set(dst: &mut u8, b: u8, indirect: bool) -> u16 {
	*dst = (1 << b) | *dst;
    if indirect { 16 } else { 8 }
}

fn res(dst: &mut u8, b: u8, indirect: bool) -> u16 {
	*dst = (!(1 << b)) & *dst;
    if indirect { 16 } else { 8 }
}

fn swap(dst: &mut u8, flags: &mut u8, indirect: bool) -> u16 {
    *dst = (*dst << 4) | (*dst >> 4);
    *flags = if *dst == 0 { FLAG_Z } else { 0 };
    if indirect { 16 } else { 8 }
}

fn sla(dst: &mut u8, flags: &mut u8, indirect: bool) -> u16 {
    *dst = *dst << 1;
    if *dst == 0 {
        *flags |= FLAG_Z;
    } else {
        *flags &= !FLAG_Z;
    }
    *flags &= !FLAG_N;
    *flags &= !FLAG_H;
    if indirect { 16 } else { 8 }
}