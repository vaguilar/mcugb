#include <stdio.h>
#include <stdint.h>

#include "tests.h"
#include "cpu.h"
#include "mem.h"

int main(int argc, char **argv) {
	FILE *fp;
	uint32_t i;

	run_tests();

	fp = fopen("SPACE.GB", "r");
	fread(MEM, 32768-1, 1, fp);
	fclose(fp);

	cpu_reset();
	REG_PC = 0x0100;

	for (i = 0; i < 25240; i++) {
		cpu_step();
		cpu_debug();
	}

}
