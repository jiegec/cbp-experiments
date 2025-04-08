# dynamorio tracer

Build:

```shell
mkdir -p build
cd build
cmake .. -DCMAKE_PREFIX_PATH=$HOME/prefix/
dynamorio
make
```

Run:

```shell
cd build
~/prefix/dynamorio/bin64/drrun -c libbrtrace.so -- command args
```