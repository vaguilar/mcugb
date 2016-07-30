#include <stdio.h>
#include <string.h>
#include <stdint.h>

#include "mem.h"
#include "cpu.h"
#include "gpu.h"

#define DEBUG 0

uint8_t MEM[65536] = {0};

uint8_t mem_read8(uint16_t addr) {
	if (DEBUG && addr == 0xff40) printf("Reading MM register, [%04x]\n", addr);
	if (DEBUG && addr == 0xff44) printf("Reading LY register, [%04x] = %02x\n", addr, MEM[addr]);
	if (addr >= 0xff00) {}
	return MEM[addr];
}

uint16_t mem_read16(uint16_t addr) {
	return (MEM[addr+1] << 8) | MEM[addr];
	//return ((uint16_t*) (&MEM[addr]))[0];
}

void mem_write8(uint16_t addr, uint8_t byte) {
	if (addr < 0x8000) {
		if (addr < 0x2000) {
			//
		} else if (addr < 0x4000) {
			// ROM bank select (0 points to one as well)
			// BANK = byte ? byte : 1;
			printf("Switching to ROM bank %d, PC = %04x\n", byte, REG_PC);

			if (byte == 0) byte = 1;
			//memcpy
		} else if (addr < 0x6000) {
			//
		} else {
			// memory mode select
			// MEMORY_MODE = byte & 1;
		}
	} else {
		if (addr == 0xff00) {
			/* joypad */
			if (byte & 0x10) {
				/* non-directional */
				MEM[0xff00] = 0xd0 | cpu_joypad_states[0];
			} else if (byte & 0x20) {
				/* directional */
				MEM[0xff00] = 0xe0 | cpu_joypad_states[1];
			}
		} else if (addr == 0xff04) {
			/* divider register */
			REG_DIV = 0;

		} else if (addr == 0xff40) {
			/* lcdc */
			REG_LCDC = byte;
			if (BIT0) {} /* display bg? */
			if (BIT1) {} /* display sprite? */

		} else if (addr == 0xff41) {
			/* lcdc stat */
			REG_STAT = byte;

		} else if (addr == 0xff46) {
			/* dma */
			REG_DMA = byte;
			mem_dma(byte << 8);

		} else {
			MEM[addr] = byte;
			if (DEBUG && addr > 0xff00) printf("Writing to MM register, [%04x] = %02x at PC = %04x\n", addr, byte, REG_PC);
		}
	}
}

void mem_write16(uint16_t addr, uint16_t word) {
	//((uint16_t*) (&MEM[addr]))[0] = word;

	//MEM[addr+1] = word >> 8;
	//MEM[addr] = word & 0xff;

	mem_write8(addr, word & 0xff);
	mem_write8(addr+1, word >> 8);
}

uint8_t mem_fetch8() {
	uint8_t byte = mem_read8(REG_PC);
	REG_PC++;
	return byte;
}

uint16_t mem_fetch16() {
	uint16_t word = mem_read16(REG_PC);
	REG_PC += 2;
	return word;
}

void mem_dma(uint16_t addr) {
	uint8_t i;
	if (DEBUG) printf("requesting DMA transfer $%04x \n", addr);
	for (i = 0; i < 40 * 4; i++) {
		MEM[0xfe00+i] = MEM[addr+i];
	}
}

void mem_load_program(uint16_t addr, uint8_t *program, uint16_t size) {
	memcpy(&MEM[addr], program, (size_t) size);
}

void mem_debug(uint16_t addr, uint16_t length) {
	uint16_t i;
	while (length) {
		printf("%04x | ", addr);
		for (i = 0; i < 16 && length; i++) {
			printf("%02x ", mem_read8(addr));
			length--;
			addr++;
		}
		printf("\n");
	}
}
