#!/bin/bash
set -x
pushd dynamorio
mkdir -p build
cd build
cmake .. -DCMAKE_PREFIX_PATH=$HOME/prefix/dynamorio
make
popd

pushd pin
for file_path in "$HOME"/pin-external-*/; do
    make clean all TARGET=intel64 PIN_ROOT=$file_path
    break
done
popd
