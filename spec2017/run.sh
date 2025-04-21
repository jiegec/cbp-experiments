#!/bin/bash
# validate that our build of SPEC INT 2017 matches our performance expectation
# usage: ./run.sh <build directory>
set -e -x

rm -rf temp
mkdir -p temp
pushd temp
cp -ar ../data/500.perlbench_r/* .
cp -v ../$1/500.perlbench_r/500.perlbench_r .
numactl -C 0 perf stat -e instructions,cycles,branches,branch-misses,task-clock ./500.perlbench_r -I./lib checkspam.pl 2500 5 25 11 150 1 1 1 1 >log
popd
