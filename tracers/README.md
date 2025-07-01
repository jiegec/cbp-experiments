# tracers

The repo contains the following tracers:

1. intel-pt: 1.4x slowdown, 0.14 bytes per branch, needs post processing at `src/bin/intel_pt.rs`
2. pin: 35x slowdown, 0.16 bytes per branch after compression
3. dynamorio: 58x slowdown, 0.16 bytes per branch after compression

Assumes that pin is installed to `~/prefix/pin`, and dynamorio is installed to `~/prefix/dynamorio`

Build tracers:

1. Install Pin to `$HOME/prefix/pin`
2. Install DynamoRIO to `$HOME/prefix/dynamorio`
3. Run:

```shell
./build.sh
```
