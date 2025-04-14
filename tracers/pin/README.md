# pin tracer

Build:

```shell
make clean all TARGET=intel64 PIN_ROOT=$HOME/prefix/pin
```

Run:

```shell
$HOME/prefix/pin/pin -t obj-intel64/brtrace.so -- command args
```

pin tracer on a trimmed leela test (5% of total):

1. slowdown: 12s -> 458s, 38x
2. storage: 3.9GB w/ compression for 1.8e10 branches (each branch takes 1.73 bit)
3. accuracy: perf says 18322617163 branches executed, actually captured 18322613546 branch executions, error less than 0.01%

