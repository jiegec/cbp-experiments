# cbp-experiments

Usage:

```shell
cargo build --release
```

Build tracers under `tracers` if necessary.

Run experiments:

1. Prepare benchmark config under `benchmarks/[config]/config.json`
2. Record traces: `cargo run --release --bin benchmark -- record [tracer] [config]`, traces are stored under `benchmarks/[config]/traces/final`
3. Run SimPoint clustering: TODO
4. Run branch prediction: TODO
