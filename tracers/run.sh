#!/bin/bash
set -x
echo "DynamoRIO tracer:" >run.log
/usr/bin/time ~/prefix/dynamorio/bin64/drrun -c ./dynamorio/build/libbrtrace.so -- $* 2>&1 | tee -a run.log
echo "Pin tracer:" >>run.log
for file_path in "$HOME"/pin-external-*/; do
    /usr/bin/time $file_path/pin -t ./pin/obj-intel64/brtrace.so -- $* 2>&1 | tee -a run.log
    break
done
