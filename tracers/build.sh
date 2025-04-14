#!/bin/bash
set -e -x
pushd dynamorio
mkdir -p build
cd build
cmake .. -DCMAKE_PREFIX_PATH=$HOME/prefix/dynamorio
make
popd

pushd pin
make clean all TARGET=intel64 PIN_ROOT=$HOME/prefix/pin
popd
