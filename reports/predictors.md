# Predictors

## TAGE-SC 192KB

- TAGE: 1169584 bits
- SC: 29531 bits
- SCGH: 141398 bits
- IMLI: 49628 bits
- LOCAL: 177840 bits

TAGE:

- 28 banks:
    - 2 way associative
    - each bank has 1024 entries
    - each entry has 3 bit counter, 2 bit useful, 14 bit tag
    - space: 28 * 1024 * (3 + 2 + 14) * 2 = 1089536
- base bimodal table: 32768 entries
    - pred: 1 bit per entry
    - hyst: 2 bit per 2 entries
    - space: 32768 * (1 + 2 / 2) = 65536
- history vector #1: 5000 bits
- history vector #2: 27 bits
- tick counter: 12 bits
- allocation counters: 9429 bits
- count miss 11 counter: 8 bits
- random number generator: 36 bits
- total entries: 28 * 1024 * 2 = 56 K
- total space: 1089536 + 65536 + 5000 + 27 + 12 + 9429 + 8 + 36 = 1169584

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

## TAGE-SC-L 8KB

- TAGE: 58165 bits
- SC: 8872 bits
- LOOP: 312 bits

TAGE:

- the banks with longer history lengths:
    - 17 banks
    - each bank has 128 entries
    - each entry has 3 bit counter, 2 bit useful, 12 bit tag
    - space: 17 * 128 * (3 + 2 + 12) = 36992
- the banks with shorter history lengths:
    - 9 banks
    - each bank has 128 entries
    - each entry has 3 bit counter, 2 bit useful, 8 bit tag
    - space: 9 * 128 * (3 + 2 + 8) = 14976
- base bimodal table: 4096 entries
    - pred: 1 bit per entry
    - hyst: 1 bit per 4 entries
    - space: 4096 * (1 + 1 / 4) = 5120
- use altpred on newly allocated counters
    - 8 entries
    - 5 bit per entry
    - space: 8 * 5 = 40 bits
- history vector #1: 1000 bits
- history vector #2: 27 bits
- tick counter: 10 bits
- total entries: 17 * 128 + 9 * 128 = 3328 = 3.25 K
- total space: 36992 + 14976 + 5120 + 40 + 1000 + 27 + 10 = 58165

SC: TODO

LOOP:

- 8 entries, per entry:
    - 10 bit tag
    - 10 bit current iteration
    - 10 bit number of iterations
    - 4 bit confidence
    - 4 bit age
    - 1 bit direction
- space: 8 * (10 + 10 + 10 + 4 + 4 + 1) = 312

## ITTAGE 64KB

- ITTAGE: 64768 bytes
- Region table: 240 bytes (752 bytes for 64-bit version)
- IUM: 384 bytes
- Total: 65392 bytes
