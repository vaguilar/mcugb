#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "cpu.h"
#include "mem.h"

void assert_equals(uint32_t expected, uint32_t actual) {
	if (expected != actual) {
		printf("Assert FAILED. Expected value: 0x%04x, actual 0x%04x\n", expected, actual);
		exit(1);
	}
}

uint8_t test0() {
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

uint8_t test1() {
	uint16_t i;
	uint8_t program[] = {
		0x3e, 0x0d, // LD A, 0x0d
		0xc6, 0x01, // ADD A, 0x01
		0xc6, 0x03, // ADD A, 0x03
		0xd6, 0x10, // SUB A, 0x10
		0x76,		// HALT
	};

	cpu_reset();
	mem_load_program(0, program, sizeof(program));

	for (i = 0; i < 5; i++) {
		cpu_step();
		cpu_debug();
	}

	return REG_A == 1;
}

/* testing rlca flags */
uint8_t test2() {
	uint16_t i;
	uint8_t program[] = {
		0x3e, 0x81, // ld a, 0x00
		0x07,		// rlca
		0x07,		// rlca
		0x76,		// halt
	};

	cpu_reset();
	mem_load_program(0, program, sizeof(program));

	for (i = 0; i < 3; i++) {
		cpu_step();
		cpu_debug();
	}

	if (REG_A == 0x06 && (REG_F | FLAG_Z) == 0) {
		printf("Register A should be 0x06, Z flag should not be set\n");
		return 0;
	}
	return 1;
}

/* testing sub */
uint8_t test3() {
	REG_AF = 0x0020;
	REG_HL = 0x0002;
	sub(&REG_L);
	assert_equals(REG_AF, 0xfe70);

	REG_AF = 0x0c50;
	REG_HL = 0x8900;
	sub(&REG_H);
	assert_equals(REG_AF, 0x8350);

	REG_AF = 0xee50;
	REG_HL = 0x8900;
	sub(&REG_H);
	assert_equals(REG_AF, 0x6540);
	return 1;
}

/* testing cp */
uint8_t test4() {
	REG_AF = 0x0020;
	REG_HL = 0x0002;
	cp(&REG_L);
	assert_equals(REG_AF, 0x0070);

	REG_AF = 0x0c50;
	REG_HL = 0x8900;
	cp(&REG_H);
	assert_equals(REG_AF, 0x0c50);

	REG_AF = 0xee50;
	REG_HL = 0x8900;
	cp(&REG_H);
	assert_equals(REG_AF, 0xee40);
	return 1;
}

/* testing daa instruction */
uint8_t test_daa() {
	REG_AF = 0x3C00;
	daa();
	assert_equals(0x42, REG_A);
	assert_equals(0x00, REG_F);

	REG_AF = 0xAA00;
	daa();
	assert_equals(0x10, REG_A);
	assert_equals(0x10, REG_F);
	return 1;
}

void run_tests() {
	char *tests_names[] = {"basicProgram", "addCorrectResult", "rclaCorrectResult", "subCorrectResult", "cpCorrectResult", "daaCorrectResult", 0};
	uint8_t (*tests[])() = {test0, test1, test2, test3, test4, test_daa, 0};
	uint8_t i, result;

	for (i = 0; tests[i]; i++) {
		cpu_reset();

		printf("Test %s: ", tests_names[i]);
		result = tests[i]();
		printf("%s\n\n", result ? "PASSED" : "FAILED");

		if (result == 0) break;
	}
}
