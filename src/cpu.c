#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#include "cpu.h"
#include "gpu.h"
#include "mem.h"

#define DEBUG 0

uint16_t TAC_SELECT[] = {1024, 16, 64, 256};

void cpu_reset() {
	memset(&registers, 0, sizeof(registers));
	memset(&cpu_joypad_states, 0xf, sizeof(cpu_joypad_states));
	REG_LCDC = 0x91;
	MEM[0xff47] = 0xfc;
	MEM[0xff48] = 0xff;
	MEM[0xff49] = 0xff;
}

void cpu_push_stack(uint16_t word) {
	REG_SP -= 2;
	mem_write16(REG_SP, word);
}

uint16_t cpu_pop_stack() {
	uint16_t result = mem_read16(REG_SP);
	REG_SP += 2;
	return result;
}

void cpu_set_interrupt(uint8_t interrupt) {
	REG_INTERRUPT_FLAG |= interrupt;
}

void cpu_unset_interrupt(uint8_t interrupt) {
	REG_INTERRUPT_FLAG &= ~interrupt;
}

/* generic instructions */
void ld_rr(uint8_t *reg_dst, uint8_t *reg_src) {
	*reg_dst = *reg_src;
}

void ld_rm(uint8_t *reg_dst, uint16_t addr_src) {
	*reg_dst = mem_read8(addr_src);
}

void ld_mr(uint16_t addr_dst, uint8_t *reg_src) {
	mem_write8(addr_dst, *reg_src);
}

void inc8(uint8_t *reg) {
	int16_t result = *reg + 1;
	cpu_set_flag(FLAG_Z, !(result & 0xff));
	cpu_set_flag(FLAG_N, 0);
	cpu_set_flag(FLAG_H, result > 0xff);
	*reg = result & 0xff;
}

void dec8(uint8_t *reg) {
	int16_t result = *reg - 1;
	int8_t result4 = (*reg & 0xf) - 1;
	cpu_set_flag(FLAG_Z, (result & 0xff) == 0);
	cpu_set_flag(FLAG_N, 1);
	cpu_set_flag(FLAG_H, result4 < 0);
	*reg = result & 0xff;
}

void inc16(uint8_t *reg_h, uint8_t *reg_l) {
	*reg_l++;
	if (*reg_l == 0x00) *reg_h++;
}

void dec16(uint8_t *reg_h, uint8_t *reg_l) {
	*reg_l--;
	if (*reg_l == 0xff) *reg_h--;
}

void sub(uint8_t *reg) {
	uint16_t result = REG_A - *reg;
	uint8_t result4 = (REG_A & 0xf) - (*reg & 0xf);
	cpu_set_flag(FLAG_Z, !(result & 0xff));
	cpu_set_flag(FLAG_N, 1);
	cpu_set_flag(FLAG_H, result4 < 0);
	cpu_set_flag(FLAG_C, result < 0);
	REG_A = result & 0xff;
}

void sbc(uint8_t *reg) {
	uint8_t byte = *reg;
	if (cpu_c()) byte++;
	uint16_t result = REG_A - byte;
	uint8_t result4 = (REG_A & 0xf) - (byte & 0xf);
	cpu_set_flag(FLAG_Z, !(result & 0xff));
	cpu_set_flag(FLAG_N, 1);
	cpu_set_flag(FLAG_H, result4 < 0);
	cpu_set_flag(FLAG_C, result < 0);
	REG_A = result & 0xff;
}

void xor(uint8_t *reg) {
	REG_A ^= *reg;
	cpu_set_flag(FLAG_Z, !REG_A);
	cpu_set_flag(FLAG_N, 0);
	cpu_set_flag(FLAG_H, 0);
	cpu_set_flag(FLAG_C, 0);
}

void add(uint8_t *reg) {
	uint16_t result = REG_A + *reg;
	uint8_t result4 = (REG_A & 0xf) + (*reg & 0xf);
	cpu_set_flag(FLAG_Z, !(result & 0xff));
	cpu_set_flag(FLAG_N, 0);
	cpu_set_flag(FLAG_H, result4 > 0xf);
	cpu_set_flag(FLAG_C, result > 0xff);
	REG_A = result;
}

void adc(uint8_t *reg) {
	uint16_t byte = *reg;
	if (cpu_c()) byte++;
	uint16_t result = REG_A + byte;
	uint8_t result4 = (REG_A & 0xf) + (byte & 0xf);
	cpu_set_flag(FLAG_Z, !(result & 0xff));
	cpu_set_flag(FLAG_N, 0);
	cpu_set_flag(FLAG_H, result4 > 0xf);
	cpu_set_flag(FLAG_C, result > 0xff);
	REG_A = result & 0xff;
}

void cp(uint8_t *reg) {
	int8_t byte = *reg;
	int16_t result = REG_A - byte;
	int16_t result4 = (REG_A & 0xf) - (byte & 0xf);
	cpu_set_flag(FLAG_Z, !(result & 0xff));
	cpu_set_flag(FLAG_N, 1);
	cpu_set_flag(FLAG_H, result4 < 0);
	cpu_set_flag(FLAG_C, result < 0);
}

void ret() {
	REG_PC = cpu_pop_stack();
}

void and(uint8_t *reg) {
	uint8_t byte = *reg;
	uint16_t result = REG_A & byte;
	cpu_set_flag(FLAG_Z, !(result & 0xff));
	cpu_set_flag(FLAG_N, 0);
	cpu_set_flag(FLAG_H, 1);
	cpu_set_flag(FLAG_C, 0);
	REG_A = result;
}

void rst(uint16_t addr) {
	cpu_push_stack(REG_PC);
	REG_PC = addr;
}

void res(uint8_t b, uint8_t *reg) {
	*reg = *reg & ~(1 << b);
}

void rrc(uint8_t *reg) {
	/* rotate right, old bit 0 to carry */
	uint8_t new_carry = *reg & 1;
	*reg = (*reg >> 1) | (new_carry ? BIT7 : 0);
	cpu_set_flag(FLAG_Z, *reg == 0);
	cpu_set_flag(FLAG_N, 0);
	cpu_set_flag(FLAG_H, 0);
	cpu_set_flag(FLAG_C, new_carry);
}

void rr(uint8_t *reg) {
	/* "9-bit" rotate right */
	uint8_t new_carry = *reg & 1;
	*reg = (*reg >> 1) | (cpu_c() ? BIT7 : 0);
	cpu_set_flag(FLAG_Z, *reg == 0);
	cpu_set_flag(FLAG_N, 0);
	cpu_set_flag(FLAG_H, 0);
	cpu_set_flag(FLAG_C, new_carry);
}

void swap(uint8_t *reg) {
	/* swap a */
	uint8_t result = (*reg << 4) | (*reg >> 4);
	cpu_set_flag(FLAG_Z, !(result & 0xff));
	cpu_set_flag(FLAG_N, 0);
	cpu_set_flag(FLAG_H, 0);
	cpu_set_flag(FLAG_C, 0);
	*reg = result & 0xff;
}

void rl(uint8_t *reg) {
	/* "9-bit" rotate left */
	uint8_t new_carry = *reg >> 7;
	*reg = (*reg << 1) | (cpu_c() ? 1 : 0);
	cpu_set_flag(FLAG_Z, *reg == 0);
	cpu_set_flag(FLAG_N, 0);
	cpu_set_flag(FLAG_H, 0);
	cpu_set_flag(FLAG_C, new_carry);
}

void cpu_update_timer(uint8_t cycles) {
	uint8_t mode = REG_TAC & (BIT0 | BIT1);
	uint8_t timer_on = REG_TAC & BIT2;
	if (timer_on) {
		cpu_state.timer_cycles += cycles;
		if (cpu_state.timer_cycles > TAC_SELECT[mode]) {
			cpu_state.timer_cycles -= TAC_SELECT[mode];
			REG_TIMA++;

			if (REG_TIMA == 0) {
				REG_TIMA = REG_TMA;
				cpu_set_interrupt(INT_TIMER);
			}
		}
	}

	cpu_state.divider_cycles += cycles;
	if (cpu_state.divider_cycles > 256) {
		cpu_state.divider_cycles -= 256;
		REG_DIV++;
	}
}

/* returns num of cycles */
uint8_t cpu_step() {
	uint16_t pc = REG_PC;
	uint8_t op = mem_fetch8();
	uint8_t cycles = 0;

	char instruction_str[32] = {0};

	if (op == 0xcb) {
		op = mem_fetch8();
		cycles = cpu_execute_cb(op, instruction_str);
	} else {
		cycles = cpu_execute(op, instruction_str);
	}

	if (DEBUG) printf("%04x: %s  | %d cycles\n", pc, instruction_str, cycles);

	if (cpu_state.interrupts) {
		if (REG_INTERRUPT_ENABLE & INT_VBLANK & REG_INTERRUPT_FLAG) {
			cpu_state.interrupts = 0;
			cpu_push_stack(REG_PC);
			REG_PC = 0x0040;
			cpu_unset_interrupt(INT_VBLANK);
		} else if (REG_INTERRUPT_ENABLE & INT_LCDC & REG_INTERRUPT_FLAG) {
			cpu_state.interrupts = 0;
			cpu_push_stack(REG_PC);
			REG_PC = 0x0048;
			cpu_unset_interrupt(INT_LCDC);
		} else if (REG_INTERRUPT_ENABLE & INT_TIMER & REG_INTERRUPT_FLAG) {
			cpu_state.interrupts = 0;
			cpu_push_stack(REG_PC);
			REG_PC = 0x0050;
			cpu_unset_interrupt(INT_TIMER);
		} else if (REG_INTERRUPT_ENABLE & INT_SERIAL & REG_INTERRUPT_FLAG) {
			cpu_state.interrupts = 0;
			cpu_push_stack(REG_PC);
			REG_PC = 0x0058;
			cpu_unset_interrupt(INT_SERIAL);
		} else if (REG_INTERRUPT_ENABLE & INT_JOYPAD & REG_INTERRUPT_FLAG) {
			cpu_state.interrupts = 0;
			cpu_push_stack(REG_PC);
			REG_PC = 0x0060;
			cpu_unset_interrupt(INT_JOYPAD);
		}
	}

	/* update timer */
	cpu_update_timer(cycles);

	return cycles;
}

uint8_t cpu_execute(uint8_t op, char* instruction_str) {
	uint8_t cycles = 0;
	int8_t offset = 0;
	int8_t immediate = 0;
	uint16_t immediate16 = 0;
	uint8_t byte = 0;
	uint16_t addr = 0;

	int32_t result = 0;
	int32_t result4 = 0; /* for half carry flag */
	int32_t result12 = 0; /* for two byte half carry flag */

	switch (op) {
	case 0x00:
		/* nop */
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "nop");
		break;
	case 0x01:
		/* ld bc, nn */
		REG_BC = mem_fetch16();
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "ld bc, $%04x", immediate16);
		break;
	case 0x02:
		/* ld (bc), a */
		mem_write8(REG_BC, REG_A);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld (bc), a");
		break;
	case 0x03:
		/* inc bc */
		REG_C++;
		if (REG_C == 0x00) REG_B++;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "inc bc");
		break;
	case 0x04:
		/* inc b */
		result = REG_B + 1;
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, result > 0xff);
		REG_B = result;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "inc b");
		break;
	case 0x05:
		/* dec b */
		result = REG_B - 1;
		result4 = (REG_B & 0xf) - 1;
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 1);
		cpu_set_flag(FLAG_H, result4 < 0);
		REG_B = result & 0xff;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "dec b");
		break;
	case 0x06:
		/* ld b, n */
		REG_B = mem_fetch8();
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld b, $%02x", REG_B);
		break;
	case 0x07:
		/* rlca */
		byte = (REG_A & BIT7) ? 1 : 0;
		result = (REG_A << 1) | byte;
		cpu_set_flag(FLAG_Z, 0); /* always reset */
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, 0);
		cpu_set_flag(FLAG_C, byte);
		REG_A = result & 0xff;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "rlca");
		break;
	case 0x08:
		/* ld (nn), sp */
		addr = mem_fetch16();
		mem_write16(addr, REG_SP);
		cycles = 20;
		if (DEBUG) sprintf(instruction_str, "ld ($%02hhx), sp", addr);
		break;
	case 0x09:
		/* add hl, bc */
		result = REG_HL + REG_BC;
		result12 = (REG_HL & 0xfff) + (REG_BC & 0xfff);
		/* z flag not affected */
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, result12 > 0xfff);
		cpu_set_flag(FLAG_C, result > 0xffff);
		REG_HL = result & 0xffff;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "add hl, bc");
		break;
	case 0x0a:
		/* ld a, (bc) */
		immediate = mem_read8(REG_BC);
		REG_A = immediate;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld a, (bc)");
		break;
	case 0x0b:
		/* dec bc */
		REG_C--;
		if (REG_C == 0xff) REG_B--;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "dec bc");
		break;
	case 0x0c:
		/* inc c */
		inc8(&REG_C);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "inc c");
		break;
	case 0x0d:
		/* dec c */
		dec8(&REG_C);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "dec c");
		break;
	case 0x0e:
		/* ld c, n */
		REG_C = mem_fetch8();
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld c, $%02x", REG_C);
		break;
	case 0x10:
		/* stop */
		/* TODO implement correctly */
		REG_PC--;
		//cpu_state.halt = 1;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "stop");
		break;
	case 0x0f:
		/* rrca */
		rrc(&REG_A);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "rrca");
		break;
	case 0x11:
		/* ld de, nn */
		immediate16 = mem_fetch16();
		REG_DE = immediate16;
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "ld de, $%04x", immediate16);
		break;
	case 0x12:
		/* ld (de), a */
		mem_write8(REG_DE, REG_A);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld (de), a");
		break;
	case 0x13:
		/* inc de */
		REG_E++;
		if (REG_E == 0x00) REG_D++;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "inc de");
		break;
	case 0x14:
		/* inc d */
		cycles = 4;
		inc8(&REG_D);
		if (DEBUG) sprintf(instruction_str, "inc d");
		break;
	case 0x15:
		/* dec d */
		result = REG_D - 1;
		result4 = (REG_D & 0xf) - 1;
		cpu_set_flag(FLAG_Z, !result);
		cpu_set_flag(FLAG_N, 1);
		cpu_set_flag(FLAG_H, result4 < 0);
		REG_D = result & 0xff;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "dec d");
		break;
	case 0x16:
		/* ld d, n */
		REG_D = mem_fetch8();
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld d, $%02x", REG_D);
		break;
	case 0x17:
		/* rla */
		rl(&REG_A);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "rla");
		break;
	case 0x18:
		/* jr n */
		offset = (int8_t) mem_fetch8(); /* signed value */
		REG_PC += (int16_t) offset;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "jr $%02hhx", offset);
		break;
	case 0x19:
		/* add hl, de */
		result = REG_HL + REG_DE;
		result4 = (REG_HL & 0xfff) + (REG_DE & 0xfff);
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, result4 >> 12);
		cpu_set_flag(FLAG_C, result >> 16);
		REG_HL = result;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "add hl, de");
		break;
	case 0x1a:
		/* ld a, (de) */
		ld_rm(&REG_A, REG_DE);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld a, (de)");
		break;
	case 0x1b:
		/* dec de */
		REG_E--;
		if (REG_E == 0xff) REG_D--;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "dec de");
		break;
	case 0x1c:
		/* inc e */
		inc8(&REG_E);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "inc e");
		break;
	case 0x1d:
		/* dec e */
		result = REG_E - 1;
		result4 = (REG_E & 0xf) - 1;
		cpu_set_flag(FLAG_Z, !result);
		cpu_set_flag(FLAG_N, 1);
		cpu_set_flag(FLAG_H, result4 < 0);
		REG_E = result & 0xff;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "dec e");
		break;
	case 0x1e:
		/* ld e, n */
		REG_E = mem_fetch8();
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld e, $%02x", REG_E);
		break;
	case 0x1f:
		/* rra */
		rr(&REG_A);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "rra");
		break;
	case 0x20:
		/* jr nz, n */
		offset = (int8_t) mem_fetch8(); /* signed value */
		if (cpu_nz()) REG_PC += (int16_t) offset;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "jr nz, $%02hhx", offset);
		break;
	case 0x21:
		/* ld hl, nn */
		REG_HL = mem_fetch16();
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "ld hl, $%04x", REG_HL);
		break;
	case 0x22:
		/* ldi (hl), a */
		mem_write8(REG_HL, REG_A);
		REG_L++;
		if (REG_L == 0x00) REG_H++;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ldi (hl), a");
		break;
	case 0x23:
		/* inc hl */
		REG_L++;
		if (REG_L == 0x00) REG_H++;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "inc hl");
		break;
	case 0x24:
		/* inc h */
		inc8(&REG_H);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "inc h");
		break;
	case 0x25:
		/* dec h */
		dec8(&REG_H);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "dec h");
		break;
	case 0x26:
		/* ld h, n */
		REG_H = mem_fetch8();
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld h, $%02x", REG_H);
		break;
	case 0x27:
		/* daa */
		/* TODO verify this */
		result = REG_A + cpu_h();
		if (result & 0x0f > 0x09) result += 0x06;
		if (result & 0xf0 > 0x90) {
			cpu_set_flag(FLAG_C, 1);
			result += 0x60;
		}
		cpu_set_flag(FLAG_Z, !result);
		cpu_set_flag(FLAG_H, 0);
		REG_A = result;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "daa");
		break;
	case 0x28:
		/* jr z, n */
		offset = (int8_t) mem_fetch8(); /* signed value */
		if (cpu_z()) REG_PC += (int16_t) offset;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "jr z, $%02hhx", offset);
		break;
	case 0x29:
		/* add hl, hl */
		result = REG_HL + REG_HL;
		result12 = (REG_HL & 0xfff) + (REG_HL & 0xfff);
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, result12 > 0xfff);
		cpu_set_flag(FLAG_C, result > 0xffff);
		REG_HL = result & 0xffff;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "add hl, hl");
		break;
	case 0x2a:
		/* ldi a, (hl) */
		REG_A = mem_read8(REG_HL);
		REG_L++;
		if (REG_L == 0x00) REG_H++;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ldi a, (hl)");
		break;
	case 0x2b:
		/* dec hl */
		REG_L--;
		if (REG_L == 0xff) REG_H--;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "dec hl");
		break;
	case 0x2c:
		/* inc l */
		inc8(&REG_L);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "inc l");
		break;
	case 0x2d:
		/* dec l */
		dec8(&REG_L);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "dec l");
		break;
	case 0x2e:
		/* ld l, n */
		byte = mem_fetch8();
		REG_L = byte;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld l, $%02hhx", byte);
		break;
	case 0x2f:
		/* cpl a */
		REG_A = ~REG_A;
		cpu_set_flag(FLAG_N, 1);
		cpu_set_flag(FLAG_H, 1);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "cpl a");
		break;
	case 0x30:
		/* jr nc, n */
		offset = (int8_t) mem_fetch8(); /* signed value */
		if (cpu_nc()) REG_PC += (int16_t) offset;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "jr nc, $%02hhx", offset);
		break;
	case 0x31:
		/* ld sp, nn */
		REG_SP = mem_fetch16();
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "ld sp, $%04x", REG_SP);
		break;
	case 0x32:
		/* ldd (hl), a */
		mem_write8(REG_HL, REG_A);
		REG_HL--;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ldd (hl), a");
		break;
	case 0x34:
		/* inc (hl) */
		result = mem_read8(REG_HL) + 1;
		mem_write8(REG_HL, result & 0xff);
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, result > 0xff);
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "inc (hl)");
		break;
	case 0x35:
		/* dec (hl) */
		byte = mem_read8(REG_HL);
		result = byte - 1;
		result4 = (byte & 0xf) - 1;
		mem_write8(REG_HL, result & 0xff);
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 1);
		cpu_set_flag(FLAG_H, result4 < 0);
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "dec (hl)");
		break;
	case 0x36:
		/* ld (hl), n */
		byte = mem_fetch8();
		mem_write8(REG_HL, byte);
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "ld (hl), $%02x", byte);
		break;
	case 0x37:
		/* scf */
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, 0);
		cpu_set_flag(FLAG_C, 1);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "scf");
		break;
	case 0x38:
		/* jr c, n */
		offset = (int8_t) mem_fetch8(); /* signed value */
		if (cpu_c()) REG_PC += (int16_t) offset;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "jr c, $%02hhx", offset);
		break;
	case 0x39:
		/* add hl, sp */
		result = REG_HL + REG_SP;
		result4 = (REG_HL & 0xfff) + (REG_SP & 0xfff);
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, result4 >> 12);
		cpu_set_flag(FLAG_C, result >> 16);
		REG_HL = result;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "add hl, sp");
		break;
	case 0x3a:
		/* ldd a, (hl) */
		REG_A = mem_read8(REG_HL);
		REG_L--;
		if (REG_L == 0xff) REG_H--;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ldd a, (hl)");
		break;
	case 0x3c:
		/* inc a */
		result = REG_A + 1;
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, result > 0xff);
		REG_A = result;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "inc a");
		break;
	case 0x3d:
		/* dec a */
		result = REG_A - 1;
		result4 = (REG_A & 0xf) - 1;
		cpu_set_flag(FLAG_Z, !result);
		cpu_set_flag(FLAG_N, 1);
		cpu_set_flag(FLAG_H, result < 0);
		REG_A = result & 0xff;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "dec a");
		break;
	case 0x3e:
		/* ld a, n */
		REG_A = mem_fetch8();
		if (DEBUG) sprintf(instruction_str, "ld a, $%02x", REG_A);
		cycles = 8;
		break;
	case 0x3f:
		/* ccf */
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, 0);
		cpu_set_flag(FLAG_C, !cpu_c());
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ccf");
		break;
	case 0x40:
		/* ld b, b */
		REG_B = REG_B;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld b, b");
		break;
	case 0x44:
		/* ld b, h */
		REG_B = REG_H;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld b, h");
		break;
	case 0x45:
		/* ld b, l */
		REG_B = REG_L;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld b, l");
		break;
	case 0x46:
		/* ld b, (hl) */
		REG_B = mem_read8(REG_HL);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld b, (hl)");
		break;
	case 0x47:
		/* ld b, a */
		REG_B = REG_A;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld b, a");
		break;
	case 0x48:
		/* ld c, b */
		REG_C = REG_B;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld c, b");
		break;
	case 0x49:
		/* ld c, c */
		REG_C = REG_C;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld c, c");
		break;
	case 0x4a:
		/* ld c, d */
		REG_C = REG_D;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld c, d");
		break;
	case 0x4b:
		/* ld c, e */
		ld_rr(&REG_C, &REG_E);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld c, e");
		break;
	case 0x4c:
		/* ld c, h */
		ld_rr(&REG_C, &REG_H);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld c, h");
		break;
	case 0x4d:
		/* ld c, l */
		ld_rr(&REG_C, &REG_L);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld c, l");
		break;
	case 0x4e:
		/* ld c, (hl) */
		REG_C = mem_read8(REG_HL);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld c, (hl)");
		break;
	case 0x4f:
		/* ld c, a */
		REG_C = REG_A;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld c, a");
		break;
	case 0x50:
		/* ld d, b */
		REG_D = REG_B;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld d, b");
		break;
	case 0x51:
		/* ld d, c */
		REG_D = REG_C;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld d, c");
		break;
	case 0x52:
		/* ld d, d */
		REG_D = REG_D;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld d, d");
		break;
	case 0x53:
		/* ld d, e */
		REG_D = REG_E;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld d, e");
		break;
	case 0x54:
		/* ld d, h */
		REG_D = REG_H;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld d, h");
		break;
	case 0x55:
		/* ld d, l */
		REG_D = REG_L;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld d, l");
		break;
	case 0x56:
		/* ld d, (hl) */
		REG_D = mem_read8(REG_HL);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld d, (hl)");
		break;
	case 0x57:
		/* ld d, a */
		REG_D = REG_A;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld d, a");
		break;
	case 0x58:
		/* ld e, b */
		REG_E = REG_B;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld e, b");
		break;
	case 0x59:
		/* ld e, c */
		REG_E = REG_C;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld e, c");
		break;
	case 0x5a:
		/* ld e, d */
		REG_E = REG_D;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld e, d");
		break;
	case 0x5b:
		/* ld e, e */
		REG_E = REG_E;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld e, e");
		break;
	case 0x5c:
		/* ld e, h */
		REG_E = REG_H;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld e, h");
		break;
	case 0x5d:
		/* ld e, l */
		REG_E = REG_L;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld e, l");
		break;
	case 0x5e:
		/* ld e, (hl) */
		REG_E = mem_read8(REG_HL);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld e, (hl)");
		break;
	case 0x5f:
		/* ld e, a */
		REG_E = REG_A;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld e, a");
		break;
	case 0x60:
		/* ld h, b */
		REG_H = REG_B;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld h, b");
		break;
	case 0x61:
		/* ld h, c */
		REG_H = REG_C;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld h, c");
		break;
	case 0x62:
		/* ld h, d */
		REG_H = REG_D;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld h, d");
		break;
	case 0x63:
		/* ld h, e */
		REG_H = REG_E;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld h, e");
		break;
	case 0x64:
		/* ld h, h */
		REG_H = REG_H;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld h, h");
		break;
	case 0x65:
		/* ld h, l */
		REG_H = REG_L;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld h, l");
		break;
	case 0x66:
		/* ld h, (hl) */
		REG_H = mem_read8(REG_HL);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld h, (hl)");
		break;
	case 0x67:
		/* ld h, a */
		REG_H = REG_A;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld h, a");
		break;
	case 0x68:
		/* ld l, c */
		REG_L = REG_C;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld l, c");
		break;
	case 0x69:
		/* ld l, c */
		REG_L = REG_C;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld l, c");
		break;
	case 0x6a:
		/* ld l, d */
		REG_L = REG_D;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld l, d");
		break;
	case 0x6b:
		/* ld l, e */
		REG_L = REG_E;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld l, e");
		break;
	case 0x6c:
		/* ld l, h */
		REG_L = REG_H;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld l, h");
		break;
	case 0x6d:
		/* ld l, l */
		REG_L = REG_L;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld l, l");
		break;
	case 0x6e:
		/* ld l, (hl) */
		REG_L = mem_read8(REG_HL);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld l, (hl)");
		break;
	case 0x6f:
		/* ld l, a */
		REG_L = REG_A;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld l, a");
		break;
	case 0x70:
		/* ld (hl), b */
		mem_write8(REG_HL, REG_B);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld (hl), b");
		break;
	case 0x71:
		/* ld (hl), c */
		mem_write8(REG_HL, REG_C);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld (hl), c");
		break;
	case 0x72:
		/* ld (hl), d */
		mem_write8(REG_HL, REG_D);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld (hl), d");
		break;
	case 0x73:
		/* ld (hl), e */
		mem_write8(REG_HL, REG_E);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld (hl), e");
		break;
	case 0x74:
		/* ld (hl), h */
		mem_write8(REG_HL, REG_H);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld (hl), h");
		break;
	case 0x75:
		/* ld (hl), l */
		mem_write8(REG_HL, REG_L);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld (hl), l");
		break;
	case 0x76:
		/* halt */
		//REG_PC--;
		//printf("halting!\n");
		//exit(1);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "halt");
		break;
	case 0x77:
		/* ld (hl), a */
		mem_write8(REG_HL, REG_A);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld (hl), a");
		break;
	case 0x78:
		/* ld a, b */
		REG_A = REG_B;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld a, b");
		break;
	case 0x79:
		/* ld a, c */
		REG_A = REG_C;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld a, c");
		break;
	case 0x7a:
		/* ld a, d */
		REG_A = REG_D;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld a, d");
		break;
	case 0x7b:
		/* ld a, e */
		REG_A = REG_E;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld a, e");
		break;
	case 0x7c:
		/* ld a, h */
		REG_A = REG_H;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld a, h");
		break;
	case 0x7d:
		/* ld a, l */
		REG_A = REG_L;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld a, l");
		break;
	case 0x7e:
		/* ld a, (hl) */
		REG_A = mem_read8(REG_HL);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld a, (hl)");
		break;
	case 0x7f:
		/* ld a, a */
		REG_A = REG_A;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld a, a");
		break;
	case 0x80:
		/* add a, b */
		add(&REG_B);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "add a, b");
		break;
	case 0x81:
		/* add a, c */
		add(&REG_C);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "add a, c");
		break;
	case 0x82:
		/* add a, d */
		add(&REG_D);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "add a, d");
		break;
	case 0x83:
		/* add a, e */
		add(&REG_E);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "add a, e");
		break;
	case 0x84:
		/* add a, h */
		add(&REG_H);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "add a, h");
		break;
	case 0x85:
		/* add a, l */
		add(&REG_L);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "add a, l");
		break;
	case 0x86:
		/* add a, (hl) */
		byte = mem_read8(REG_HL);
		result = REG_A + byte;
		result4 = (REG_A & 0xf) * (byte & 0xf);
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, result4 > 0xf);
		cpu_set_flag(FLAG_C, result > 0xff);
		REG_A = result;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "add a, (hl)");
		break;
	case 0x87:
		/* add a, a */
		add(&REG_A);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "add a, a");
		break;
	case 0x88:
		/* adc a, b */
		adc(&REG_B);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "adc a, b");
		break;
	case 0x89:
		/* adc a, c */
		adc(&REG_C);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "adc a, c");
		break;
	case 0x8a:
		adc(&REG_D);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "adc a, d");
		break;
	case 0x8b:
		adc(&REG_E);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "adc a, e");
		break;
	case 0x8c:
		adc(&REG_H);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "adc a, h");
		break;
	case 0x8d:
		adc(&REG_L);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "adc a, l");
		break;
	case 0x8e:
		/* adc a, (hl) */
		byte = mem_read8(REG_HL);
		if (cpu_c()) byte++;
		result = REG_A + byte;
		result4 = (REG_A & 0xf) + (byte & 0xf);
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, result4 > 0xf);
		cpu_set_flag(FLAG_C, result > 0xff);
		REG_A = result;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "add a, (hl)");
		break;
	case 0x90:
		/* sub a, b */
		sub(&REG_B);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "sub a, b");
		break;
	case 0x91:
		/* sub a, c */
		sub(&REG_C);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "sub a, c");
		break;
	case 0x92:
		/* sub a, d */
		sub(&REG_D);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "sub a, d");
		break;
	case 0x93:
		/* sub a, e */
		sub(&REG_E);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "sub a, e");
		break;
	case 0x94:
		/* sub a, h */
		sub(&REG_H);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "sub a, h");
		break;
	case 0x95:
		/* sub a, l */
		sub(&REG_L);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "sub a, l");
		break;
	case 0x96:
		/* sub a, (hl) */
		byte = mem_read8(REG_HL);
		result = REG_A - byte;
		result4 = (REG_A & 0xf) - (byte & 0xf);
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 1);
		cpu_set_flag(FLAG_H, result4 < 0);
		cpu_set_flag(FLAG_C, result < 0);
		REG_A = result & 0xff;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "sub a, (hl)");
		break;
	case 0x97:
		/* sub a, a */
		sub(&REG_A);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "sub a, a");
		break;
	case 0x98:
		/* sbc a, b */
		sbc(&REG_B);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "sbc a, b");
		break;
	case 0x99:
		/* sbc a, c */
		sbc(&REG_C);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "sbc a, c");
		break;
	case 0x9a:
		/* sbc a, d */
		sbc(&REG_D);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "sbc a, d");
		break;
	case 0x9b:
		/* sbc a, h */
		sbc(&REG_E);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "sbc a, e");
		break;
	case 0x9c:
		/* sbc a, h */
		sbc(&REG_H);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "sbc a, h");
		break;
	case 0x9e:
		/* sbc a, (hl) */
		byte = mem_read8(REG_HL);
		if (cpu_c()) byte++;
		result = REG_A - byte;
		result4 = (REG_A & 0xf) - (byte & 0xf);
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 1);
		cpu_set_flag(FLAG_H, result4 < 0);
		cpu_set_flag(FLAG_C, result < 0);
		REG_A = result & 0xff;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "sbc a, (hl)");
		break;
	case 0x9f:
		/* sbc a, a */
		sbc(&REG_A);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "sbc a, a");
		break;
	case 0xa0:
		/* and a, b */
		and(&REG_B);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "and a, b");
		break;
	case 0xa1:
		/* and a, c */
		and(&REG_C);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "and a, c");
		break;
	case 0xa2:
		/* and a, d */
		and(&REG_D);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "and a, d");
		break;
	case 0xa3:
		/* and a, e */
		and(&REG_E);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "and a, e");
		break;
	case 0xa7:
		/* and a, a */
		and(&REG_A);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "and a, a");
		break;
	case 0xa8:
		/* xor a, b */
		xor(&REG_B);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "xor a, b");
		break;
	case 0xa9:
		/* xor a, c */
		xor(&REG_C);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "xor a, c");
		break;
	case 0xaf:
		/* xor a, a */
		xor(&REG_A);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "xor a, a");
		break;
	case 0xb0:
		/* or b */
		REG_A |= REG_B;
		cpu_set_flag(FLAG_Z, !(REG_A & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, 0);
		cpu_set_flag(FLAG_C, 0);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "or b");
		break;
	case 0xb1:
		/* or c */
		REG_A |= REG_C;
		cpu_set_flag(FLAG_Z, !(REG_A & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, 0);
		cpu_set_flag(FLAG_C, 0);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "or c");
		break;
	case 0xb2:
		/* or d */
		REG_A |= REG_D;
		cpu_set_flag(FLAG_Z, !(REG_A & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, 0);
		cpu_set_flag(FLAG_C, 0);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "or d");
		break;
	case 0xb3:
		/* or e */
		REG_A |= REG_E;
		cpu_set_flag(FLAG_Z, !(REG_A & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, 0);
		cpu_set_flag(FLAG_C, 0);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "or e");
		break;
	case 0xb4:
		/* or h */
		REG_A |= REG_H;
		cpu_set_flag(FLAG_Z, !(REG_A & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, 0);
		cpu_set_flag(FLAG_C, 0);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "or h");
		break;
	case 0xb5:
		/* or l */
		REG_A |= REG_L;
		cpu_set_flag(FLAG_Z, REG_A == 0);
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, 0);
		cpu_set_flag(FLAG_C, 0);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "or l");
		break;
	case 0xb6:
		/* or (hl) */
		REG_A |= MEM[REG_HL];
		cpu_set_flag(FLAG_Z, REG_A == 0);
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, 0);
		cpu_set_flag(FLAG_C, 0);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "or (hl)");
		break;
	case 0xb7:
		/* or a */
		REG_A |= REG_A;
		cpu_set_flag(FLAG_Z, !(REG_A & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, 0);
		cpu_set_flag(FLAG_C, 0);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "or a");
		break;
	case 0xb8:
		/* cp a, b */
		byte = REG_B;
		result = REG_A - byte;
		result4 = (REG_A & 0xf) - (byte & 0xf);
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 1);
		cpu_set_flag(FLAG_H, result4 < 0);
		cpu_set_flag(FLAG_C, result < 0);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "cp a, b");
		break;
	case 0xb9:
		/* cp a, c */
		byte = REG_C;
		result = REG_A - byte;
		result4 = (REG_A & 0xf) - (byte & 0xf);
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 1);
		cpu_set_flag(FLAG_H, result4 < 0);
		cpu_set_flag(FLAG_C, result < 0);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "cp a, c");
		break;
	case 0xba:
		/* cp a, d */
		cp(&REG_D);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "cp a, d");
		break;
	case 0xbe:
		/* cp a, (hl) */
		byte = mem_read8(REG_HL);
		result = REG_A - byte;
		result4 = (REG_A & 0xf) - (byte & 0xf);
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 1);
		cpu_set_flag(FLAG_H, result4 < 0);
		cpu_set_flag(FLAG_C, result < 0);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "cp a, (hl)");
		break;
	case 0xc0:
		/* ret nz */
		if (cpu_nz()) REG_PC = cpu_pop_stack();
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ret nz");
		break;
	case 0xc1:
		/* pop bc */
		REG_BC = cpu_pop_stack();
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "pop bc");
		break;
	case 0xc2:
		/* jp nz, nn */
		addr = mem_fetch16();
		if (cpu_nz()) REG_PC = addr;
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "jp nz, $%04x", addr);
		break;
	case 0xc3:
		/* jp nn */
		REG_PC = mem_fetch16();
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "jp $%04x", REG_PC);
		break;
	case 0xc4:
		/* call nz, nn */
		addr = mem_fetch16();
		if (cpu_nz()) {
			cpu_push_stack(REG_PC);
			REG_PC = addr;
		}
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "call nz, $%04x", addr);
		break;
	case 0xc5:
		/* push bc */
		cpu_push_stack(REG_BC);
		cycles = 16;
		if (DEBUG) sprintf(instruction_str, "push bc");
		break;
	case 0xc6:
		/* add a, # */
		immediate = mem_fetch8();
		result = REG_A + immediate;
		result4 = (REG_A & 0xf) + (immediate & 0xf);
		cpu_set_flag(FLAG_Z, !result);
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, result4 > 0xf);
		cpu_set_flag(FLAG_C, result > 0xff);
		REG_A = result;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "add a, $%02x", immediate);
		break;
	case 0xc8:
		/* ret z */
		if (cpu_z()) REG_PC = cpu_pop_stack();
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ret z");
		break;
	case 0xc9:
		/* ret */
		REG_PC = cpu_pop_stack();
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ret");
		break;
	case 0xca:
		/* jp z, nn */
		addr = mem_fetch16();
		if (cpu_z()) REG_PC = addr;
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "jp z, $%04x", addr);
		break;
	case 0xcb:
		/* extra cb instructions */
		byte = mem_fetch8();
		cycles = cpu_execute_cb(byte, instruction_str);
		break;
	case 0xcd:
		/* call nn */
		addr = mem_fetch16();
		cpu_push_stack(REG_PC);
		REG_PC = addr;
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "call $%04x", addr);
		break;
	case 0xce:
		/* adc a, # */
		byte = mem_fetch8();
		if (cpu_c()) byte++;
		result = REG_A + byte;
		result4 = (REG_A & 0xf) + (byte & 0xf);
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, result4 > 0xf);
		cpu_set_flag(FLAG_C, result > 0xff);
		REG_A = result;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "add a, a");
		break;
	case 0xd0:
		/* ret nc */
		if (cpu_nc()) REG_PC = cpu_pop_stack();
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ret nc");
		break;
	case 0xd1:
		/* pop de */
		REG_DE = cpu_pop_stack();
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "pop de");
		break;
	case 0xd2:
		/* jp nc, nn */
		addr = mem_fetch16();
		if (cpu_nc()) REG_PC = addr;
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "jp nc, $%04x", addr);
		break;
	case 0xd4:
		/* call nc, nn */
		addr = mem_fetch16();
		if (cpu_nc()) {
			cpu_push_stack(REG_PC);
			REG_PC = addr;
		}
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "call nc, $%04x", addr);
		break;
	case 0xd7:
		/* rst $10 */
		rst(0x10);
		cycles = 32;
		if (DEBUG) sprintf(instruction_str, "rst $10");
		break;
	case 0xd6:
		/* sub a, # */
		immediate = mem_fetch8();
		result = REG_A - immediate;
		result4 = (REG_A & 0xf) - (immediate & 0xf);
		cpu_set_flag(FLAG_Z, !result);
		cpu_set_flag(FLAG_N, 1);
		cpu_set_flag(FLAG_H, result4 < 0);
		cpu_set_flag(FLAG_C, result < 0);
		REG_A = result;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "sub a, $%02x", immediate);
		break;
	case 0xd8:
		/* ret c */
		if (cpu_c()) ret();
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ret c");
		break;
	case 0xd5:
		/* push de */
		cpu_push_stack(REG_DE);
		cycles = 16;
		if (DEBUG) sprintf(instruction_str, "push de");
		break;
	case 0xd9:
		/* reti */
		REG_PC = cpu_pop_stack();
		cpu_state.interrupts = 1;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "reti");
		break;
	case 0xda:
		/* jp c, nn */
		addr = mem_fetch16();
		if (cpu_c()) REG_PC = addr;
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "jp c, $%04x", addr);
		break;
	case 0xde:
		/* sbc a, # */
		byte = mem_fetch8();
		if (cpu_c()) byte++;
		result = REG_A - byte;
		result4 = (REG_A & 0xf) - (byte & 0xf);
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 1);
		cpu_set_flag(FLAG_H, result4 < 0);
		cpu_set_flag(FLAG_C, result < 0);
		REG_A = result & 0xff;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "sbc a, $%02x", cpu_c() ? byte-1 : byte);
		break;
	case 0xdf:
		/* rst $18 */
		cpu_push_stack(REG_PC);
		REG_PC = 0x18;
		cycles = 32;
		if (DEBUG) sprintf(instruction_str, "rst $18");
		break;
	case 0xe0:
		/* ld ($ff00+n), a */
		byte = mem_fetch8();
		mem_write8(0xff00 + byte, REG_A);
		//mem_debug((0xff00 + immediate) & 0xfff0, 16);
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "ld ($ff00+$%02hhx), a", byte);
		break;
	case 0xe1:
		/* pop hl */
		REG_HL = cpu_pop_stack();
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "pop hl");
		break;
	case 0xe2:
		/* ld ($ff00+c), a */
		mem_write8(0xff00 + REG_C, REG_A);
		//mem_debug((0xff00 + REG_C) & 0xfff0, 16);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld ($ff00+c), a");
		break;
	case 0xe5:
		/* push hl */
		cpu_push_stack(REG_HL);
		cycles = 16;
		if (DEBUG) sprintf(instruction_str, "push hl");
		break;
	case 0xe6:
		/* and a, # */
		byte = mem_fetch8();
		result = REG_A & byte;
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, 1);
		cpu_set_flag(FLAG_C, 0);
		REG_A = result;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "and a, $%02x", byte);
		break;
	case 0xe7:
		/* rst $20 */
		cpu_push_stack(REG_PC);
		REG_PC = 0x20;
		cycles = 32;
		if (DEBUG) sprintf(instruction_str, "rst $20");
		break;
	case 0xe8:
		/* add sp, # */
		immediate = mem_fetch8(); /* signed */
		result = (REG_SP & 0xff) + immediate;
		result4 = (REG_SP & 0x0f) + (immediate & 0x0f);
		cpu_set_flag(FLAG_Z, 0);
		cpu_set_flag(FLAG_N, 0);
		if (immediate < 0) {
			cpu_set_flag(FLAG_H, result4 > (immediate & 0xf));
			cpu_set_flag(FLAG_C, result > immediate);
		} else {
			cpu_set_flag(FLAG_H, result4 > 0xf);
			cpu_set_flag(FLAG_C, result > 0xff);
		}
		REG_SP += immediate;
		cycles = 16;
		if (DEBUG) sprintf(instruction_str, "add sp, $%02x", immediate);
		break;
	case 0xe9:
		/* jp hl */
		/* i've seen this instruction as jp (hl) but the parenthesis seem misleading
		 * since we aren't dereferencing hl, leaving them out */
		REG_PC = REG_HL;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "jp hl");
		break;
	case 0xea:
		/* ld (nn), a */
		addr = mem_fetch16();
		mem_write8(addr, REG_A);
		cycles = 16;
		if (DEBUG) sprintf(instruction_str, "ld ($%04x), a", addr);
		break;
	case 0xee:
		/* xor # */
		byte = mem_fetch8();
		xor(&byte);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "xor $%02x", byte);
		break;
	case 0xef:
		/* rst $28 */
		rst(0x28);
		cycles = 32;
		if (DEBUG) sprintf(instruction_str, "rst $28");
		break;
	case 0xf0:
		/* ld a, ($ff00+n) */
		byte = mem_fetch8();
		REG_A = mem_read8(0xff00 + byte);
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "ld a, ($ff00+$%02hhx), %04x", byte, 0xff00 + byte);
		break;
	case 0xf1:
		/* pop af */
		REG_AF = cpu_pop_stack();
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "pop af");
		break;
	case 0xf3:
		/* disable int */
		cpu_state.interrupts = 0;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "di");
		break;
	case 0xf5:
		/* push af */
		cpu_push_stack(REG_AF);
		cycles = 16;
		if (DEBUG) sprintf(instruction_str, "push af");
		break;
	case 0xf6:
		/* or a, # */
		byte = mem_fetch8();
		REG_A |= byte;
		cpu_set_flag(FLAG_Z, !(REG_A & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, 0);
		cpu_set_flag(FLAG_C, 0);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "or a, $%02x", byte);
		break;
	case 0xf7:
		/* rst $30 */
		cpu_push_stack(REG_PC);
		REG_PC = 0x30;
		cycles = 32;
		if (DEBUG) sprintf(instruction_str, "rst $30");
		break;
	case 0xf8:
		/* ldhl sp, n */
		immediate = mem_fetch8();
		result = (REG_SP & 0xff) + immediate;
		result4 = (REG_SP & 0xf) + (immediate & 0xf);
		cpu_set_flag(FLAG_Z, 0);
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, result4 > 0xf);
		cpu_set_flag(FLAG_C, result > 0xff);
		REG_HL = REG_SP + (int8_t) immediate;
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "ldhl sp+$%02x", immediate);
		break;
	case 0xf9:
		/* ld sp, hl */
		REG_SP = REG_HL;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld sp, hl");
		break;
	case 0xfa:
		/* ld a, (nn) */
		addr = mem_fetch16();
		REG_A = mem_read8(addr);
		cycles = 16;
		if (DEBUG) sprintf(instruction_str, "ld a, ($%04x)", addr);
		break;
	case 0xfb:
		/* enable int */
		cpu_state.interrupts = 1;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ei");
		break;
	case 0xfe:
		/* cp a, n */
		byte = mem_fetch8();
		result = REG_A - byte;
		result4 = (REG_A & 0xf) - (byte & 0xf);
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 1);
		cpu_set_flag(FLAG_H, result4 < 0);
		cpu_set_flag(FLAG_C, result < 0);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "cp a, $%02hx", byte);
		break;
	case 0xff:
		/* rst $38 */
		cpu_push_stack(REG_PC);
		REG_PC = 0x38;
		cycles = 32;
		if (DEBUG) sprintf(instruction_str, "rst $38");
		break;

	default:
		printf("Unimplemented instruction %02hx at $%04x\n", op, REG_PC - 1);
		exit(1);
		break;
	}

	return cycles;
}

void rlc(uint8_t *reg) {
	uint8_t carry = *reg >> 7;
	*reg = (*reg << 1) | carry;
	cpu_set_flag(FLAG_Z, *reg == 0);
	cpu_set_flag(FLAG_N, 0);
	cpu_set_flag(FLAG_H, 0);
	cpu_set_flag(FLAG_C, carry);
}

void srl(uint8_t *reg) {
	uint8_t new_carry = *reg & 1;
	*reg = *reg >> 1;
	*reg = *reg & ~BIT7; /* TODO might not be necessary */
	cpu_set_flag(FLAG_Z, *reg == 0);
	cpu_set_flag(FLAG_N, 0);
	cpu_set_flag(FLAG_H, 0);
	cpu_set_flag(FLAG_C, new_carry);
}

void sla(uint8_t *reg) {
	uint8_t result = REG_C << 1;
	cpu_set_flag(FLAG_Z, !(result & 0xff));
	cpu_set_flag(FLAG_N, 0);
	cpu_set_flag(FLAG_H, 0);
	cpu_set_flag(FLAG_C, *reg & BIT7);
	*reg = result;
}

void bit(uint8_t b, uint8_t *dst) {
	uint8_t result = (1 << b) & *dst;
	cpu_set_flag(FLAG_Z, !result);
	cpu_set_flag(FLAG_N, 0);
	cpu_set_flag(FLAG_H, 1);
}

void set(uint8_t b, uint8_t *dst) {
	*dst = (1 << b) | *dst;
}

uint8_t cpu_execute_cb(uint8_t op, char* instruction_str) {
	uint32_t result;
	uint8_t byte;
	uint8_t cycles;

	switch(op) {
	case 0x01:
		/* rlc c */
		rlc(&REG_C);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "rlc c");
		break;
	case 0x05:
		/* rlc l */
		rlc(&REG_L);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "rlc l");
		break;
	case 0x10:
		/* rl b */
		rl(&REG_B);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "rl b");
		break;
	case 0x11:
		/* rl c */
		rl(&REG_C);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "rl c");
		break;
	case 0x12:
		/* rl d */
		rl(&REG_D);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "rl d");
		break;
	case 0x13:
		/* rl e */
		rl(&REG_E);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "rl e");
		break;
	case 0x14:
		/* rl h */
		rl(&REG_H);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "rl h");
		break;
	case 0x18:
		/* rr b */
		rr(&REG_B);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "rr b");
		break;
	case 0x19:
		/* rr c */
		rr(&REG_C);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "rr c");
		break;
	case 0x1c:
		/* rr h */
		rr(&REG_H);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "rr h");
		break;
	case 0x1d:
		/* rr l */
		rr(&REG_L);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "rr l");
		break;
	case 0x21:
		/* sla c */
		result = REG_C << 1;
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, 0);
		cpu_set_flag(FLAG_C, REG_C & BIT7);
		REG_C = result & 0xff;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "sla c");
		break;
	case 0x27:
		/* sla a */
		sla(&REG_A);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "sla a");
		break;
	case 0x33:
		/* swap e */
		swap(&REG_E);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "swap e");
		break;
	case 0x37:
		/* swap a */
		result = (REG_A << 4) | (REG_A >> 4);
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, 0);
		cpu_set_flag(FLAG_C, 0);
		REG_A = result & 0xff;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "swap a");
		break;
	case 0x3a:
		/* srl d */
		srl(&REG_D);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "srl d");
		break;
	case 0x3b:
		/* srl e */
		srl(&REG_E);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "srl e");
		break;
	case 0x3c:
		/* srl h */
		srl(&REG_H);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "srl h");
		break;
	case 0x3d:
		/* srl l */
		srl(&REG_L);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "srl l");
		break;
	case 0x3f:
		/* srl a */
		srl(&REG_A);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "srl a");
		break;
	case 0x40:
		/* bit 0, b */
		bit(0, &REG_B);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "bit 0, b");
		break;
	case 0x47:
		/* bit 0, a */
		bit(1, &REG_A);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "bit 1, a");
		break;
	case 0x48:
		/* bit 0, b */
		bit(1, &REG_B);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "bit 1, b");
		break;
	case 0x50:
		/* bit 2, b */
		bit(2, &REG_B);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "bit 2, b");
		break;
	case 0x57:
		/* bit 2, a */
		bit(2, &REG_A);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "bit 2, a");
		break;
	case 0x58:
		/* bit 3, b */
		bit(3, &REG_B);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "bit 3, b");
		break;
	case 0x5e:
		/* bit 3, (hl) */
		bit(3, &MEM[REG_HL]);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "bit 3, a");
		break;
	case 0x5f:
		/* bit 3, a */
		bit(3, &REG_A);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "bit 3, a");
		break;
	case 0x60:
		/* bit 4, b */
		bit(4, &REG_B);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "bit 4, b");
		break;
	case 0x61:
		/* bit 4, c */
		bit(4, &REG_C);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "bit 4, c");
		break;
	case 0x68:
		/* bit 5, b */
		bit(5, &REG_B);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "bit 5, b");
		break;
	case 0x69:
		/* bit 5, c */
		bit(5, &REG_C);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "bit 5, c");
		break;
	case 0x6f:
		/* bit 5, a */
		bit(5, &REG_A);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "bit 5, a");
		break;
	case 0x77:
		/* bit 6, a */
		bit(6, &REG_A);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "bit 6, a");
		break;
	case 0x78:
		/* bit 7, b */
		bit(7, &REG_B);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "bit 7, b");
		break;
	case 0x7a:
		/* bit 7, d */
		bit(7, &REG_D);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "bit 7, d");
		break;
	case 0x7e:
		/* bit 7, (hl) */
		bit(7, &MEM[REG_HL]);
		cycles = 16;
		if (DEBUG) sprintf(instruction_str, "bit 7, (hl)");
		break;
	case 0x7f:
		/* bit 7, a */
		bit(7, &REG_A);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "bit 7, a");
		break;
	case 0x86:
		/* res 0, (hl) */
		res(0, &MEM[REG_HL]);
		cycles = 16;
		if (DEBUG) sprintf(instruction_str, "res 0, (hl)");
		break;
	case 0x87:
		/* res 0, a */
		res(0, &REG_A);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "res 0, a");
		break;
	case 0xc7:
		/* set 0, r */
		set(0, &REG_A);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "set 0, a");
		break;
	case 0xfe:
		/* set 7, (hl) */
		set(7, &MEM[REG_HL]);
		cycles = 16;
		if (DEBUG) sprintf(instruction_str, "set 7, (hl)");
		break;
	default:
		printf("Unimplemented instruction cb %02hx at $%04x\n", op, REG_PC - 2);
		exit(1);
		break;
	}
	return cycles;
}

void cpu_set_joypad(uint8_t directional, uint8_t button) {
	uint8_t mask = (1 << button);
	if (cpu_joypad_states[directional] & mask) {
		cpu_joypad_states[directional] &= ~mask;
		cpu_set_interrupt(INT_JOYPAD);
	}
}

void cpu_unset_joypad(uint8_t directional, uint8_t button) {
	cpu_joypad_states[directional] |= 1 << button;
}

void cpu_debug() {
	printf("AF: %02X %02X  ", REG_A, REG_F);
	printf("BC: %02X %02X\n", REG_B, REG_C);
	printf("DE: %02X %02X  ", REG_D, REG_E);
	printf("HL: %02X %02X\n", REG_H, REG_L);
	printf("SP: %02X %02X  ", REG_SP >> 8, REG_SP & 0xFF);
	printf("PC: %02X %02X\n", REG_PC >> 8, REG_PC & 0xFF);
	printf("ZNHC\n%01X%01X%01X%01X\n", cpu_z(), cpu_o(), cpu_h(), cpu_c());
	printf("\n");
}

void cpu_debug_stack() {
	uint32_t stack_ptr = REG_SP;
	uint8_t i = 0;

	printf("Stack:\n");
	for (i = 0; i < 16 && stack_ptr < 0xffff; i++) {
		printf("[%04hx]: %04hx\n", stack_ptr, mem_read16(stack_ptr));
		stack_ptr += 2;
	}
}
