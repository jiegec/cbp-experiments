# dynamorio tracer

Build:

```shell
mkdir -p build
cd build
cmake .. -DCMAKE_PREFIX_PATH=$HOME/prefix/dynamorio
make
```

Run:

```shell
cd build
~/prefix/dynamorio/bin64/drrun -c libbrtrace.so -- command args
```

dynamorio tracer on a trimmed leela test (5% of total):

1. slowdown: 12s -> 703s, 59x
2. storage: 3.9GB w/ compression (each branch takes 1.73 bit)
3. accuracy: perf says 18322617163 branches executed, actually captured 18322013216 branch executions, error less than 0.01%
