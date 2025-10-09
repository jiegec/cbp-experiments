#!/bin/bash
set -e -x
pushd dynamorio
mkdir -p build
cd build
cmake .. -DCMAKE_PREFIX_PATH=$HOME/prefix/dynamorio
make
popd

ARCH=$(uname -m)

if [ "$ARCH" = "x86_64" ]; then
    pushd pin
    pushd zstd
    mkdir -p cmakebuild
    pushd cmakebuild
    # Release: NDEBUG, disable assertions
    # disable multithread support to avoid using pthread
    cmake ../build/cmake -DCMAKE_BUILD_TYPE=Release -DZSTD_MULTITHREAD_SUPPORT=OFF
    make -j
    popd
    popd
    make clean all TARGET=intel64 PIN_ROOT=$HOME/prefix/pin
    popd
fi

pushd common
gcc dump-vdso.c -o dump-vdso
./dump-vdso > vdso
popd
