# mcugb
mcugb is a Gameboy emulator and debugger written in C for *nix operating systems. It's only dependencies are SDL and pthreads. 

### Usage:
./mcugb rom.gb

### Debugger commands:
* _r_ - Run ROM
* _c_ - Continue execution
* _Esc_ - Pause execution
* _l_ - List breakpoints
* _b_ - Set breakpoint at hex address. Example: b 1234
* _d_ - Remove breakpoint at hex address. Example: r 1234
* _s_ - Step, execute one instruction
* _x_ - Exit

### Keys:
* _x_ - A button
* _z_ - B button
* _c_ - Select
* _v_ - Start
* _Arrow keys_ - Directional Pad
