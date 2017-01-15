#include <stdio.h>
#include <stdint.h>
#include <pthread.h>
#include <string.h>
#include <stdlib.h>

#include "debugger.h"
#include "cpu.h"

uint16_t breakpoints[MAX_BREAKPOINTS] = {0};
uint8_t breakpoints_len = 0;
char cmd[32] = {0};
extern volatile uint8_t RUNNING;
extern volatile uint8_t STEP;
pthread_mutex_t mutex;
pthread_cond_t condA;

void* debugger_main(void *running) {
	pthread_mutex_init(&mutex, NULL);
	RUNNING = 1; // comment to start in debug mode
	while (1) {
        while (RUNNING == 1)
			pthread_cond_wait(&condA, &mutex);

		/* prompt for debugging command */
		printf("\n> ");
		fgets(cmd, 32, stdin);
		if (debugger_cmd(cmd)) {
			cpu_step();
			debugger_set_state(1);
		}
	}
}

void debugger_set_state(uint8_t state) {
	pthread_mutex_lock(&mutex);
	RUNNING = state;
	pthread_cond_signal(&condA);
	pthread_mutex_unlock(&mutex);
}

uint8_t debugger_cmd(char *cmd) {
	uint16_t addr = 0;	
	uint16_t ret = 0;
	char *tok;
	tok = strtok(cmd, " ");

	switch(*tok) {
	case 'b':
		/* add breakpoint */
		tok = strtok(0, " ");
		if (tok == 0) {
			printf("Missing address.\n");
		} else {
			addr = strtoul(tok, 0, 16);
			if (addr == 0) {
				printf("Invalid address.\n");
			} else {
				printf("Added breakpoint at address $%04hx.\n", addr);
				debugger_add_breakpoint(addr);
			}
		}
		break;

	case 'c':
		/* continue */
		ret = 1;
		break;

	case 'd':
		/* delete breakpoint */
		tok = strtok(0, " ");
		if (tok == 0) {
			printf("Missing address.\n");
		} else {
			addr = strtoul(tok, 0, 16);
			if (addr == 0) {
				printf("Invalid address.\n");
			} else {
				if (debugger_remove_breakpoint(addr)) {
					printf("Removed breakpoint at address $%04hx.\n", addr);
				} else {
					printf("Breakpoint at address $%04hx not found.\n", addr);
				}
			}
		}
		break;

	case 'h':
		/* help */
		debugger_help();
		break;

	case 'i':
		/* info (list register) */
		debugger_info();
		break;

	case 'l':
		/* list break points */
		debugger_list_breakpoints();
		break;

	case 'r':
		/* run */
		ret = 1;
		break;

	case 's':
		/* step */
		STEP = 1;
		ret = 1;
		break;

	case 'x':
		/* exit */
		exit(0);
		break;

	default:
		printf("Invalid command, type 'h' for list of commands.\n");
	}

	return ret;
}

void debugger_help() {
	printf("COMMANDS:\n");
	printf("  b -- Add breakpoint e.g. b 12ab\n");
	printf("  c -- Continue rom execution\n");
	printf("  d -- Delete breakpoint e.g. d 12ab\n");
	printf("  h -- This menu\n");
	printf("  i -- CPU info\n");
	printf("  l -- List breakpoints\n");
	printf("  r -- Run rom\n");
	printf("  s -- Step, execute one instruction\n");
	printf("  x -- Exit\n");
}

void debugger_list_breakpoints() {
	uint8_t i = 0;
	while (breakpoints[i] != 0) {
		printf("[%d] $%04hx\n", i, breakpoints[i]);
		i++;
	}
	if (i == 0) {
		printf("No breakpoints.\n");
	}
}

void debugger_add_breakpoint(uint16_t addr) {
	if (breakpoints_len < MAX_BREAKPOINTS) {
		breakpoints[breakpoints_len] = addr;
		breakpoints_len++;
	}
}

uint8_t debugger_remove_breakpoint(uint16_t addr) {
	uint8_t i;
	for(i = 0; i < MAX_BREAKPOINTS && breakpoints[i] != addr; i++);

	if (addr && breakpoints[i] == addr) {
		breakpoints[i] = breakpoints[breakpoints_len - 1];
		breakpoints[breakpoints_len - 1] = 0;
		breakpoints_len--;
		return 1;
	}
	return 0;
}

uint8_t debugger_in_breakpoints(uint16_t addr) {
	uint8_t i;
	for (i = 0; i < breakpoints_len; i++) {
		if (breakpoints[i] == addr) return 1;
	}
	return 0;
}

void debugger_info() {
	cpu_debug();
}
