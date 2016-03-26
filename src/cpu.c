#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#include "cpu.h"
#include "mem.h"

void cpu_reset() {
	memset(&registers, 0, sizeof(registers));
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

/* returns num of cycles */
uint8_t cpu_step() {
	uint16_t pc = REG_PC;
	uint8_t op = mem_fetch8();
	uint8_t cycles = 0;

	int8_t offset = 0;
	int8_t immediate = 0;
	uint16_t immediate16 = 0;
	uint8_t byte = 0;
	uint16_t addr = 0;

	int32_t result = 0;
	int32_t result4 = 0; /* for half carry flag */

	char instruction_str[32] = {0};

	switch (op) {
	case 0x00:
		/* nop */
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "nop");
		break;
	case 0x05:
		/* dec b */
		result = REG_B - 1;
		result4 = (REG_B & 0xf) - 1;
		cpu_set_flag(FLAG_Z, !result);
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
	case 0x0c:
		/* inc c */
		result = REG_C + 1;
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, result > 0xff);
		REG_C = result;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "inc c");
		break;
	case 0x0d:
		/* dec c */
		result = REG_C - 1;
		cpu_set_flag(FLAG_Z, !result);
		cpu_set_flag(FLAG_N, 1);
		cpu_set_flag(FLAG_H, result > REG_C);
		REG_C = result & 0xff;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "dec c");
		break;
	case 0x0e:
		/* ld c, n */
		REG_C = mem_fetch8();
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld c, $%02x", REG_C);
		break;
	case 0x11:
		/* ld de, nn */
		immediate16 = mem_fetch16();
		REG_DE = immediate16;
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "ld de, $%04x", immediate16);
		break;
	case 0x18:
		/* jr n */
		offset = (int8_t) mem_fetch8(); /* signed value */
		REG_PC += (int16_t) offset;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "jr $%02hhx", offset);
		break;
	case 0x1b:
		/* dec de */
		REG_E--;
		if (REG_E == 0xff) REG_D--;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "dec de");
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
		REG_HL++;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ldi (hl), a");
		break;
	case 0x2a:
		/* ldi a, (hl) */
		REG_A = mem_read8(REG_HL);
		REG_HL++;
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ldi a, (hl)");
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
	case 0x76:
		/* halt */
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "halt");
		break;
	case 0x78:
		/* ld a, b */
		REG_A = REG_B;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld a, b");
		break;
	case 0x7a:
		/* ld a, d */
		REG_A = REG_D;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "ld a, d");
		break;
	case 0x87:
		/* add a, a */
		result = REG_A * 2;
		result4 = (REG_A & 0xf) * 2;
		cpu_set_flag(FLAG_Z, !(result & 0xff));
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, result4 > 0xf);
		cpu_set_flag(FLAG_C, result > 0xff);
		REG_A = result;
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "add a, a");
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
	case 0xc3:
		/* jmp nn */
		REG_PC = mem_fetch16();
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "jmp $%04x", REG_PC);
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
	case 0xc9:
		/* ret */
		REG_PC = cpu_pop_stack();
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ret");
		break;
	case 0xcd:
		/* call nn */
		addr = mem_fetch16();
		cpu_push_stack(REG_PC);
		REG_PC = addr;
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "call $%04x", addr);
		break;
	case 0xd0:
		/* ret nc */
		if (cpu_nc()) REG_PC = cpu_pop_stack();
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ret nc");
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

	case 0xaf:
		/* xor a, a */
		REG_A ^= REG_A;
		cpu_set_flag(FLAG_Z, !REG_A);
		cpu_set_flag(FLAG_N, 0);
		cpu_set_flag(FLAG_H, 0);
		cpu_set_flag(FLAG_C, 0);
		cycles = 4;
		if (DEBUG) sprintf(instruction_str, "xor a, a");
		break;

	case 0xe0:
		/* ld ($ff00+n), a */
		immediate = mem_fetch8();
		mem_write8(0xff00 + immediate, REG_A);
		mem_debug((0xff00 + immediate) & 0xfff0, 16);
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "ld ($ff00+$%02x), a", immediate);
		break;
	case 0xe2:
		/* ld ($ff00+c), a */
		mem_write8(0xff00 + REG_C, REG_A);
		mem_debug((0xff00 + REG_C) & 0xfff0, 16);
		cycles = 8;
		if (DEBUG) sprintf(instruction_str, "ld ($ff00+c), a");
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
		if (DEBUG) sprintf(instruction_str, "and $%02x", byte);
		break;
	case 0xea:
		/* ld (nn), a */
		addr = mem_fetch16();
		mem_write8(addr, REG_A);
		cycles = 16;
		if (DEBUG) sprintf(instruction_str, "ld ($%04x), a", addr);
		break;
	case 0xf0:
		/* ld a, ($ff00+n) */
		immediate = mem_fetch8();
		REG_A = mem_read8(0xff00 + immediate);
		cycles = 12;
		if (DEBUG) sprintf(instruction_str, "ld a, ($ff00+$%02x)", immediate);
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

	default:
		printf("Unimplemented instruction %02hx\n", op);
		exit(1);
		break;
	}

	printf("%04x: %s  | %d cycles\n", pc, instruction_str, cycles);

	if (cpu_state.interrupts) {
		if (REG_INTERRUPT_ENABLE & INT_VBLANK) {}
	}
	return cycles;
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
