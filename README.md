# Giggle Flop

A toy ISA designed and simulated from scratch.

## Memory System

- Configurable cache
    - Variable number of levels
    - Custom access latency and capacity per level
    - Cache line length configurable
- Write-through no-allocate scheme
- Direct mapped cache

## CPU

- 5 stage pipeline
- No-Pipeline Mode

## Assembler

- Assembles custom assembly language code to simulator machine code
- Helpful error messages with line numbers as appropriate
- Single line C-style comments supported
- Named labels supported

## Misc

- GUI debugger
- Breakpoints
- Single step execution
- "Running" execution
- Verbose logging
