# cbp-experiments

A platform to run conditional branch prediction experiments on real binaries. It can:

1. Capture branch trace from real binaries
2. Run SimPoint methodology on branch traces to reduce size and simulation speed
3. Simulate branch predictors (from CBP2016, etc.) on the branch traces

## Usage

1. Clone this repository, and:

```shell
cargo build --release
```

2. Build tracers under `tracers` directory if necessary. See [tracers/README.md](./tracers/README.md) for details.
3. Run experiments:
    1. Prepare benchmark config under `benchmarks/[config]/config.json`, see [Configuration section](#configuration)
    2. Record traces: `cargo run --release --bin benchmark -- record --tracer [tracer] --config-name [config]`, traces are stored under `benchmarks/[config]/traces/final`
    3. Display trace statistics: `cargo run --release --bin benchmark -- info --config-name [config]`
    4. Run SimPoint clustering: `cargo run --release --bin benchmark -- simpoint --config-name [config] --size [instructions]`
    4. Run branch prediction: `cargo run --release --bin benchmark -- simulate --config-name [config] --predictor [predictor]`
    5. Find results under: `benchmarks/[config]/[simulate]/[datetime]-[predictor]/per-benchmark` or use `cargo run --release --bin benchmark -- report`

## Example

Example #1 leela benchmark:

```shell
cargo run --release --bin benchmark -- record --config-name leela --tracer intel-pt 
cargo run --release --bin benchmark -- info --config-name leela
cargo run --release --bin benchmark -- simpoint --config-name leela --size 100000000
cargo run --release --bin benchmark -- simulate --config-name leela --predictor AndreSeznec-TAGE-SC-L-8KB
```

Example #2 some dynamically linked binaries:

```shell
cargo run --release --bin benchmark -- record --config-name test --tracer intel-pt 
cargo run --release --bin benchmark -- info --config-name test
cargo run --release --bin benchmark -- simpoint --config-name test --size 1000
cargo run --release --bin benchmark -- simulate --config-name test --predictor AndreSeznec-TAGE-SC-L-8KB
```

Example #3 full spec int 2017 rate:

```shell
# it takes 20+ hours
cargo run --release --bin benchmark -- record --config-name spec-int-2017-rate --tracer pin
cargo run --release --bin benchmark -- info --config-name spec-int-2017-rate
cargo run --release --bin benchmark -- simpoint --config-name spec-int-2017-rate --size 100000000
cargo run --release --bin benchmark -- simulate --config-name spec-int-2017-rate --predictor AndreSeznec-TAGE-SC-L-8KB
```

## Configuration

Configuration hierarchy:

1. config, e.g. `spec-int-2017-rate-1`
2. benchmark, e.g. `leela`
3. command, e.g. `leela ref.sgf`
4. simpoint, e.g. the first simpoint phase of `leela ref.sgf`

You can add benchmarks to `benchmarks/[config]/config.json`, like:

```json
{
    "benchmarks": [{
        "name": "ls",
        "commands": [{
            "command": "/usr/bin/ls ."
        }]
    }, {
        "name": "cat",
        "commands": [{
            "command": "/usr/bin/cat /proc/self/maps"
        }]
    }]
}
```

## FIXME

fork breaks intel pt converter due to missing filtering of events from child processes
