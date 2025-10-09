# dynamorio tracer

Build and install dynamorio to `~/prefix/dynamorio`:

```shell
git clone git@github.com:DynamoRIO/dynamorio.git
cd dynamorio
git submodule update --init --recursive
mkdir build
cd build
cmake .. -DCMAKE_INSTALL_PREFIX=$HOME/prefix/dynamorio
make -j8
make install
```

Build tracer:

```shell
sudo apt install -y libzstd-dev
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
