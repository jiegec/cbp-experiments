#!/bin/bash
set -e -x
pushd dynamorio
mkdir -p build
cd build
cmake .. -DCMAKE_PREFIX_PATH=$HOME/prefix/dynamorio
make
popd

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
