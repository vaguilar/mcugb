#pragma once

#define MAX_BREAKPOINTS 32

void* debugger_main(void *running);
void debugger_set_state(uint8_t state);
uint8_t debugger_cmd(char *cmd);
void debugger_help();
void debugger_list_breakpoints();
void debugger_add_breakpoint(uint16_t addr);
uint8_t debugger_remove_breakpoint(uint16_t addr);
uint8_t debugger_in_breakpoints(uint16_t addr);
void debugger_start();
void debugger_stop();
