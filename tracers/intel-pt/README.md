# intel-pt tracer

Usage:

```shell
sudo sysctl kernel.perf_event_paranoid=-1
perf record -e intel_pt//u command args
```

intel-pt tracer:

1. slowdown: 12s -> 17s
2. storage: 2.6GB for 1.8e10 branches (each branch takes 1.16 bit)
3. convert to custom trace format: 2.6GB -> 2.8GB, 246s, 21x slowdown overall
