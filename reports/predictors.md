# Predictors

## TAGE-SC-L 64KB

- TAGE: 463917 bits
- SC: 58190 bits
- LOOP: 1248 bits

TAGE:

- the banks with longer history lengths:
    - 20 banks
    - each bank has 1024 entries
    - each entry has 3 bit counter, 1 bit useful, 12 bit tag
    - space: 20 * 1024 * (3 + 1 + 12) = 327680
- the banks with shorter history lengths:
    - 10 banks
    - each bank has 1024 entries
    - each entry has 3 bit counter, 1 bit useful, 8 bit tag
    - space: 10 * 1024 * (3 + 1 + 8) = 122880
- base bimodal table: 8192 entries
    - pred: 1 bit per entry
    - hyst: 1 bit per 4 entries
    - space: 8192 * (1 + 1 / 4) = 10240
- use altpred on newly allocated counters
    - 16 entries
    - 5 bit per entry
    - space: 80 bits
- history vector #1: 3000 bits
- history vector #2: 27 bits
- tick counter: 10 bits
- total entries: 20 * 1024 + 10 * 1024 = 32720 = 30 K
- total space: 327680 + 122880 + 10240 + 80 + 3000 + 27 + 10 = 463917

SC: TODO

LOOP:

- 32 entries, per entry:
    - 10 bit tag
    - 10 bit current iteration
    - 10 bit number of iterations
    - 4 bit confidence
    - 4 bit age
    - 1 bit direction
- space: 32 * (10 + 10 + 10 + 4 + 4 + 1) = 1248
