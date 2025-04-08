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

dynamorio tracer:

1. slowdown: 12s -> 694s, 58x
2. storage: 35GB for 1.8e10 branches (each branch takes 2 bytes) w/o compression, 2.8GB w/ compression
