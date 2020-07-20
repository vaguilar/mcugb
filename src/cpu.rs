use crate::memory::Memory;

pub enum Interrupt {
    VBlank  = 0x01,
    LCDC    = 0x02,
    Timer   = 0x04, 
    Serial  = 0x08,
    JoyPad  = 0x10,
}

pub struct CPU {
    pub pc: u16,
    pub sp: u16,
    pub mem: Memory,
    pub registers: Registers,

    pub halt: bool,
    pub interrupts: bool,
    pub interrupt_enable: bool,
    pub interrupt_flag: bool,
    pub timer_cycles: u16,
    pub divider_cycles: u16,

    pub joypad_states: [u8; 2],
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
    pub fn new(mem: Memory) -> CPU {
        CPU {
            pc: 0x0100,
            sp: 0xffff,
            mem: mem,
            registers: Default::default(), 
            halt: false,
            interrupts: false,
            interrupt_enable: false,
            interrupt_flag: false,
            timer_cycles: 0,
            divider_cycles: 0,
            joypad_states: [0, 0],
        }
    }

    pub fn push_stack(&mut self, value: u16) {
        self.sp -= 2;
        self.mem.write16(self.sp, value);
    }

    pub fn pop_stack(&mut self) -> u16 {
        let result = self.mem.read16(&self.sp);
        self.sp += 2;
        result
    }

    pub fn set_interrupt(&mut self, interrupt: Interrupt) {
        self.mem.data[0xff0f] |= interrupt as u8;
    }

    pub fn unset_interrupt(&mut self, interrupt: Interrupt) {
        self.mem.data[0xff0f] &= !(interrupt as u8);
    }

    pub fn interrupt_reg(&self) -> u8 {
        self.mem.data[0xffff]
    }

    pub fn fetch8(&mut self) -> u8 {
        let result = self.mem.read8(&self.pc);
        self.pc += 1;
        result
    }

    pub fn step(&mut self) -> u16 {
        let op = self.fetch8();

        let mut cycles: u16 = 0;

        if op == 0xcb {
            let op2 = self.fetch8();
            cycles = self.execute_cb(op2);
        } else {
            cycles = self.execute(op);
        }

        let interrupt_flag = &self.mem.data[0xff0f];
        let interrupt_enable = &self.mem.data[0xffff];
        if self.interrupts {
            if *interrupt_enable & Interrupt::VBlank as u8 & *interrupt_flag != 0 {
                self.interrupts = false;
                self.push_stack(self.pc);
                self.pc = 0x0040;
                self.unset_interrupt(Interrupt::VBlank);
            } else if *interrupt_enable & Interrupt::LCDC as u8 & *interrupt_flag != 0 {
                self.interrupts = false;
                self.push_stack(self.pc);
                self.pc = 0x0048;
                self.unset_interrupt(Interrupt::LCDC);
            } else if *interrupt_enable & Interrupt::Timer as u8 & *interrupt_flag != 0 {
                self.interrupts = false;
                self.push_stack(self.pc);
                self.pc = 0x0050;
                self.unset_interrupt(Interrupt::Timer);
            } else if *interrupt_enable & Interrupt::Serial as u8 & *interrupt_flag !=0 {
                self.interrupts = false;
                self.push_stack(self.pc);
                self.pc = 0x0058;
                self.unset_interrupt(Interrupt::Serial);
            } else if *interrupt_enable & Interrupt::JoyPad as u8 & *interrupt_flag != 0 {
                self.interrupts = false;
                self.push_stack(self.pc);
                self.pc = 0x0060;
                self.unset_interrupt(Interrupt::JoyPad);
            }
        }

        /* update timer */
        self.update_timer(cycles);
        cycles
    }

    fn update_timer(&mut self, cycles: u16) {
        static TAC_SELECT: [u16; 4] = [1024, 16, 64, 256];

        let tac = *self.mem.reg_tac();

        let mode: usize = tac as usize & 0b00000011;
        let timer_on: u8 = tac & 0b00000010;

        if timer_on != 0 {
            self.timer_cycles += cycles;
            if self.timer_cycles > TAC_SELECT[mode] {
                self.timer_cycles -= TAC_SELECT[mode];
                *self.mem.reg_tima() += 1;

                if *self.mem.reg_tima() == 0 {
                    *self.mem.reg_tima() = *self.mem.reg_tma();
                    self.set_interrupt(Interrupt::Timer);
                }
            }
        }

        self.divider_cycles += cycles;
        if self.divider_cycles > 255 {
            self.divider_cycles -= 255;
            *self.mem.reg_div() += 1;
        }
    }

    fn execute(&self, op: u8) -> u16 {
        0
    }

    fn execute_cb(&self, op: u8) -> u16 {
        0
    }
}
