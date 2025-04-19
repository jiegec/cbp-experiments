# SPEC INT 2017 Rate-1

- Platform: Intel i9-14900K
- Compiler: GCC 12.2.0
- Optimization Flags: -O3
- Tracer: Pin

## Tracing

Summary:

| Benchmark       | Sub-benchmark | Branch Executions | Trace Size | Space per Branch | Execution Time w/o Pin | Execution Time w/Pin | Time Penalty |
|-----------------|---------------|-------------------|------------|------------------|------------------------|----------------------|--------------|
| 500.perlbench_r | checkspam     | 2.40e11           | 8.87 GiB   | 0.32 bit         | 59s                    | 6334s                | 107x         |
| 500.perlbench_r | diffmail      | 1.49e11           | 2.78 GiB   | 0.16 bit         | 33s                    | 4615s                | 140x         |
| 500.perlbench_r | splitmail     | 1.33e11           | 1.49 GiB   | 0.10 bit         | 31s                    | 3385s                | 109x         |
| 500.perlbench_r | Summary       | 5.22e11           | 13.14 GiB  | 0.22 bit         | 123s                   | 14334s               | 117x         |
| 502.gcc_r       | gcc-pp -O3    | 4.50e10           | 3.28 GiB   | 0.63 bit         | 17s                    | 1625s                | 96x          |
| 502.gcc_r       | gcc-pp -O2    | 5.37e10           | 3.46 GiB   | 0.55 bit         | 20s                    | 1930s                | 97x          |
| 502.gcc_r       | gcc-smaller   | 5.51e10           | 2.84 GiB   | 0.44 bit         | 21s                    | 1830s                | 87x          |
| 502.gcc_r       | ref32 -O5     | 4.22e10           | 1.20 GiB   | 0.24 bit         | 16s                    | 1369s                | 86x          |
| 502.gcc_r       | ref32 -O3     | 4.80e10           | 1.50 GiB   | 0.27 bit         | 24s                    | 2209s                | 92x          |
| 502.gcc_r       | Summary       | 2.44e11           | 12.24 GiB  | 0.43 bit         | 98s                    | 8963s                | 91x          |
| 505.mcf_r       | N/A           | 2.21e11           | 31.0 GiB   | 1.20 bit         | 168s                   | 4800s                | 29x          |
| 520.omnetpp_r   | N/A           | 2.15e11           | 13.3 GiB   | 0.53 bit         | 135s                   | 7289s                | 54x          |
| 523.xalancbmk_r | N/A           | 3.27e11           | 4.45 GiB   | 0.12 bit         | 112s                   | 8883s                | 79x          |
| 525.x264_r      | pass 1        | 1.44e10           | 579 MiB    | 0.34 bit         | 14s                    | 348s                 | 25x          |
| 525.x264_r      | pass 2        | 4.42e10           | 2.30 GiB   | 0.45 bit         | 39s                    | 1202s                | 31x          |
| 525.x264_r      | seek 500      | 4.78e10           | 2.77 GiB   | 0.50 bit         | 41s                    | 1258s                | 31x          |
| 525.x264_r      | Summary       | 1.06e11           | 5.64 GiB   | 0.46 bit         | 94s                    | 2808s                | 30x          |
| 531.deepsjeng_r | N/A           | 2.74e11           | 31.6 GiB   | 0.99 bit         | 140s                   | 8093s                | 58x          |
| 541.leela_r     | N/A           | 3.38e11           | 75.6 GiB   | 1.92 bit         | 224s                   | 8894s                | 40x          |
| 548.exchange2_r | N/A           | 3.01e11           | 26.3 GiB   | 0.75 bit         | 88s                    | 6753s                | 77x          |
| 557.xz_r        | cld           | 5.08e10           | 9.16 GiB   | 1.55 bit         | 60s                    | 1252s                | 21x          |
| 557.xz_r        | cpu2006docs   | 1.84e11           | 7.80 GiB   | 0.36 bit         | 65s                    | 3923s                | 60x          |
| 557.xz_r        | input         | 7.96e10           | 10.5 GiB   | 1.14 bit         | 55s                    | 1842s                | 33x          |
| 557.xz_r        | Summary       | 3.14e11           | 27.5 GiB   | 0.75 bit         | 180s                   | 7017s                | 39x          |
| Summary         | N/A           | 2.86e12           | 241 GiB    | 0.72 bit         | 1362s                  | 77834s               | 57x          |

## Simulation

MPKI of different predictors:

| benchmark       | TAGE-SC-L 8KB | TAGE-SC-L 8KB Only TAGE | TAGE-SC-L 64KB | TAGE-SC-L 64KB Only TAGE | TAGE-Cookbook |
|-----------------|---------------|-------------------------|----------------|--------------------------|---------------|
| 500.perlbench_r | 1.0033        | 1.0464                  | 0.7209         | 0.7676                   | 0.7774        |
| 502.gcc_r       | 4.6880        | 4.8908                  | 3.3600         | 3.6323                   | 3.5698        |
| 505.mcf_r       | 13.0168       | 14.0096                 | 12.2277        | 13.3399                  | 12.3923       |
| 520.omnetpp_r   | 4.0858        | 4.3132                  | 3.4897         | 4.1885                   | 3.8276        |
| 523.xalancbmk_r | 0.8520        | 0.9810                  | 0.6817         | 0.8793                   | 0.8664        |
| 525.x264_r      | 0.7596        | 0.7965                  | 0.5879         | 0.6571                   | 0.6332        |
| 531.deepsjeng_r | 4.5924        | 4.7104                  | 3.4531         | 3.8108                   | 3.5525        |
| 541.leela_r     | 11.7913       | 12.4307                 | 9.4221         | 10.1996                  | 9.6553        |
| 548.exchange2_r | 2.9609        | 3.6632                  | 1.2488         | 1.9521                   | 1.7593        |
| 557.xz_r        | 4.6811        | 5.0213                  | 4.0627         | 4.5943                   | 4.2971        |
| average         | 4.8431        | 5.1863                  | 3.9255         | 4.4021                   | 4.1331        |
| space           | 67349 bits    | 58165 bits              | 523355 bits    | 463917 bits              | 558273 bits   |

Accuracy of SimPoint versus full simulation using TAGE-SC-L 8KB:

| Benchmark  | Sub-benchmark | Full MPKI | SimPoint MPKI |
|------------|---------------|-----------|---------------|
| 525.x264_r | pass 1        | 0.5162    | 0.5090        |
