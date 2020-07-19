pub struct CPU {
    pc: u16,
    registers: CPURegisters,
    halt: bool,
    interrupts: bool,
    interrupt_enable: bool,
    interrupt_flag: bool,
    timer_cycles: u16,
    divider_cycles: u16,
}

pub struct CPURegisters {
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
            registers: CPURegisters {
                f: 0,
                a: 0,
                b: 0,
                c: 0,
                d: 0,
                e: 0,
                h: 0,
                l: 0,
            },
            halt: false,
            interrupts: false,
            interrupt_enable: false,
            interrupt_flag: false,
            timer_cycles: 0,
            divider_cycles: 0,
        }
    }
}
