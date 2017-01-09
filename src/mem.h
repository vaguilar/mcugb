#pragma once

#define REG_JOYPAD				MEM[0xff00]
#define REG_DIV					MEM[0xff04]
#define REG_TIMA				MEM[0xff05]
#define REG_TMA					MEM[0xff06]
#define REG_TAC					MEM[0xff07]
#define REG_INTERRUPT_FLAG		MEM[0xff0f]
#define REG_INTERRUPT_ENABLE	MEM[0xffff]

uint8_t  mem_read8(uint16_t addr);
uint16_t mem_read16(uint16_t addr);
void mem_write8(uint16_t addr, uint8_t byte);
void mem_write16(uint16_t addr, uint16_t word);

uint8_t mem_fetch8();
uint16_t mem_fetch16();
void mem_dma(uint16_t addr);
void mem_load_program(uint16_t addr, uint8_t *program, uint16_t size);
void mem_debug(uint16_t, uint16_t);

uint8_t MEM[65536];
