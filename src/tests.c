#include <stdio.h>
#include <stdint.h>

#include "cpu.h"
#include "mem.h"

uint32_t test0() {
	uint8_t program[] = {
		0x3e, 0xff, // ld  a, 0xff
		0xc6, 0x03, // add a, 0x03
		0xd6, 0x01,	// sub a, 0x01
		0x3d,		// dec a
		0x3d,		// dec a
		0x76,		// halt
	};

	cpu_reset();
	mem_load_program(0, program, sizeof(program));

	cpu_step();
	cpu_debug();
	if (REG_A != 0xff) {
		printf("Register A should be 0xff\n");
		return 0;
	}

	cpu_step();
	cpu_debug();
	if (REG_A != 0x02 || REG_F != (FLAG_H | FLAG_C)) {
		printf("Register A should be 0x02, H and C flag should be set\n");
		return 0;
	}

	cpu_step();
	cpu_debug();
	if (REG_A != 0x01 || REG_F != FLAG_N) {
		printf("Register A should be 0x01, N flag should be set\n");
		return 0;
	}

	cpu_step();
	cpu_debug();
	if (REG_A != 0x00 || REG_F != (FLAG_Z | FLAG_N)) {
		printf("Register A should be 0x00, Z and N flags should be set\n");
		return 0;
	}

	cpu_step();
	cpu_debug();
	if (REG_A != 0xff || REG_F != (FLAG_N | FLAG_H)) {
		printf("Register A should be 0xff, N and H flags should be set\n");
		return 0;
	}

	return 1;
}

uint32_t test1() {
	uint8_t program[] = {
		0x3e, 0x0d, // LD A, 0x0d
		0xc6, 0x01, // ADD A, 0x01
		0xc6, 0x03, // ADD A, 0x03
		0xd6, 0x10, // SUB A, 0x10
		0x76,		// HALT
	};
	uint16_t i;

	cpu_reset();
	mem_load_program(0, program, sizeof(program));

	for (i = 0; i < 5; i++) {
		cpu_step();
		cpu_debug();
	}

	return REG_A == 1;
}

void run_tests() {
	uint32_t result;

	result = test0();
	printf("test0 %s\n\n", result ? "PASSED" : "FAILED");

	result = test1();
	printf("test1 %s\n\n", result ? "PASSED" : "FAILED");
}
