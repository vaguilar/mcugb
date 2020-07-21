use crate::memory::Memory;

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
    r8: Registers8,
    r16: Registers16,
}

#[derive(Default)]
pub struct Registers8 {
    f: u8,
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
}

#[derive(Default)]
pub struct Registers16 {
    af: u16,
    bc: u16,
    de: u16,
    hl: u16,
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

    pub fn push_stack(&mut self, mem: &mut Memory, value: u16) {
        self.sp -= 2;
        mem.write16(self.sp, value);
    }

    pub fn pop_stack(&mut self, mem: &mut Memory) -> u16 {
        let result = mem.read16(self.sp);
        self.sp += 2;
        result
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
        static TAC_SELECT: [u16; 4] = [1024, 16, 64, 256];

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
        match op {
            0x00 => 0,
            0x01 => {
                // ld bc, nn
                self.reg.r16.bc = self.fetch16(mem);
                0
            }
            0x01 => {
                // ld (bc), a
                mem.write8(self.reg.r16.bc, self.reg.r8.a);
                self.reg.r16.bc = self.fetch16(mem);
                0
            }
            0xaf => {
                // xor a, a
                self.reg.r8.a = 0;
                4
            }
            0xc3 => {
                // jp nn
                self.pc = self.fetch16(mem);
                eprintln!("PC is now 0x{:04X}", self.pc);
                12
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
