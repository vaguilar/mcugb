#include <stdio.h>
#include <stdint.h>

#include "tests.h"
#include "cpu.h"
#include "mem.h"

int main(int argc, char **argv) {
	FILE *fp;
	uint32_t i, cycles, total_cycles;

	run_tests();

	fp = fopen("SPACE.GB", "r");
	fread(MEM, 32768-1, 1, fp);
	fclose(fp);

	cpu_reset();
	REG_PC = 0x0100;

	for (i = 0; i < 250000; i++) {
		//if (REG_PC == 0x0370) { printf("BREAK\n"); break; }
		cycles = cpu_step();
		gpu_step(cycles);
		cpu_debug();

		total_cycles += cycles;
	}

	mem_debug(0x9c00, 32);
}
