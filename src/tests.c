#include <stdio.h>
#include <stdint.h>

#include "cpu.h"
#include "mem.h"

int test1() {
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
	printf("test1 %s\n", test1() ? "PASSED" : "FAILED");
	printf("\n");
}
