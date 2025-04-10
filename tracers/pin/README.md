# pin tracer

Build:

```shell
make clean all TARGET=intel64 PIN_ROOT=$HOME/pin-external-x.xx/
```

Run:

```shell
$HOME/pin-external-x.xx/pin -t obj-intel64/brtrace.so -- command args
```

pin tracer on a trimmed leela test (5% of total):

1. slowdown: 12s -> 429s, 35x
2. storage: 2.8GB w/ compression for 1.8e10 branches (each branch takes 1.24 bit)
3. accuracy: perf says 18322617163 branches executed, actually captured 18322613546 branch executions, error less than 0.01%

