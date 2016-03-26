#pragma once

#define REG_AF (*((uint16_t*)&registers.F))
#define REG_BC (*((uint16_t*)&registers.C))
#define REG_DE (*((uint16_t*)&registers.E))
#define REG_HL (*((uint16_t*)&registers.L))
#define REG_PC registers.PC
#define REG_SP registers.SP

#define REG_A registers.A
#define REG_F registers.F
#define REG_B registers.B
#define REG_C registers.C
#define REG_D registers.D
#define REG_E registers.E
#define REG_H registers.H
#define REG_L registers.L

#define REG_INTERRUPT_ENABLE MEM[0xffff]

#define INT_PIN    0x10
#define INT_SERIAL 0x08
#define INT_TIMER  0x04
#define INT_LCDC   0x02
#define INT_VBLANK 0x01

#define FLAG_Z 0x80
#define FLAG_N 0x40
#define FLAG_H 0x20
#define FLAG_C 0x10

#define cpu_z() ((REG_F & FLAG_Z) != 0)
#define cpu_nz() ((REG_F & FLAG_Z) == 0)
#define cpu_c() ((REG_F & FLAG_C) != 0)
#define cpu_nc() ((REG_F & FLAG_C) == 0)
#define cpu_o() ((REG_F & FLAG_N) != 0)
#define cpu_h() ((REG_F & FLAG_H) != 0)
#define cpu_set_flag(flag, val) (REG_F = val ? REG_F | flag : REG_F & ~flag)

struct {
	uint8_t F;
	uint8_t A;

	uint8_t C;
	uint8_t B;

	uint8_t E;
	uint8_t D;

	uint8_t L;
	uint8_t H;

	uint16_t PC;
	uint16_t SP;
} registers;

struct {
	uint8_t halt;
	uint8_t interrupts;
	uint8_t interrupt_enable;
	uint8_t interrupt_flag;
} cpu_state;

void cpu_reset();
void cpu_debug();
void cpu_push_stack(uint16_t word);
uint16_t cpu_pop_stack();

void cpu_set_flags8(uint8_t prev, uint8_t curr, uint8_t subtraction);
void cpu_set_flags16(uint16_t prev, uint16_t curr, uint8_t subtraction);

uint8_t cpu_step();
