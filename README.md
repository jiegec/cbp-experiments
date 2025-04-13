# cbp-experiments

Usage:

```shell
cargo build --release
```

Build tracers under `tracers` if necessary.

Run experiments:

1. Prepare benchmark config under `benchmarks/[config]/config.json`
2. Record traces: `cargo run --release --bin benchmark -- record --tracer [tracer] --config-name [config]`, traces are stored under `benchmarks/[config]/traces/final`
3. Display trace statistics: `cargo run --release --bin benchmark -- info --config-name [config]`
3. Run SimPoint clustering: `cargo run --release --bin benchmark -- simpoint --config-name [config] --size [instructions]`
4. Run branch prediction: `cargo run --release --bin benchmark -- simulate --config-name [config] --predictor [predictor]`

Configuration hierarchy:

1. config, e.g. `spec-int-2017-rate-1`
2. benchmark, e.g. `leela`
3. command, e.g. `leela ref.sgf`
4. simpoint, e.g. the first simpoint phase of `leela ref.sgf`
