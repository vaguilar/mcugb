#include <stdio.h>
#include <string.h>
#include <stdint.h>

#include "mem.h"
#include "cpu.h"

uint8_t MEM[65536] = {0};

uint8_t mem_read8(uint16_t addr) {
	return MEM[addr];
}

uint16_t mem_read16(uint16_t addr) {
	//return (MEM[addr+1] << 8) | MEM[addr];
	return ((uint16_t*) (&MEM[addr]))[0];
}

void mem_write8(uint16_t addr, uint8_t byte) {
	MEM[addr] = byte;
}

void mem_write16(uint16_t addr, uint16_t word) {
	((uint16_t*) (&MEM[addr]))[0] = word;
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

void mem_load_program(uint16_t addr, uint8_t *program, uint16_t size) {
	memcpy(&MEM[addr], program, (size_t) size);
}
