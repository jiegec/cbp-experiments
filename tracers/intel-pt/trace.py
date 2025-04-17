# extract taken branch trace from perf output
# usage: perf script --itrace=b -i /path/to/perf.data | python3 trace.py
import sys

for line in sys.stdin:
    line = line.strip()
    parts = list(filter(lambda s: len(s) > 0, line.split(" ")))
    from_addr = parts[6]
    if from_addr == "0":
        continue
    idx = parts.index("=>")
    to_addr = parts[idx+1]
    if to_addr == "0":
        continue
    print(from_addr, to_addr)