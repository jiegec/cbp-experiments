# SPEC INT 2017 Rate-1

## benchmarks/spec-int-2017-rate-o3

- Platform: Intel i9-14900K
- Compiler: GCC 12.2.0
- Optimization Flags: -O3
- Tracer: Pin

### Tracing

Summary:

| Benchmark       | Sub-benchmark | Br. Exec. | Trace Size | Space per Br. | Exec. Time w/o Pin | Exec. Time w/ Pin | Overhead |
|-----------------|---------------|-----------|------------|---------------|--------------------|-------------------|----------|
| 500.perlbench_r | checkspam     | 2.40e11   | 8.87 GiB   | 0.32 bit      | 59s                | 6334s             | 107x     |
| 500.perlbench_r | diffmail      | 1.49e11   | 2.78 GiB   | 0.16 bit      | 33s                | 4615s             | 140x     |
| 500.perlbench_r | splitmail     | 1.33e11   | 1.49 GiB   | 0.10 bit      | 31s                | 3385s             | 109x     |
| 500.perlbench_r | Total         | 5.22e11   | 13.14 GiB  | 0.22 bit      | 123s               | 14334s            | 117x     |
| 502.gcc_r       | gcc-pp -O3    | 4.50e10   | 3.28 GiB   | 0.63 bit      | 17s                | 1625s             | 96x      |
| 502.gcc_r       | gcc-pp -O2    | 5.37e10   | 3.46 GiB   | 0.55 bit      | 20s                | 1930s             | 97x      |
| 502.gcc_r       | gcc-smaller   | 5.51e10   | 2.84 GiB   | 0.44 bit      | 21s                | 1830s             | 87x      |
| 502.gcc_r       | ref32 -O5     | 4.22e10   | 1.20 GiB   | 0.24 bit      | 16s                | 1369s             | 86x      |
| 502.gcc_r       | ref32 -O3     | 4.80e10   | 1.50 GiB   | 0.27 bit      | 24s                | 2209s             | 92x      |
| 502.gcc_r       | Total         | 2.44e11   | 12.24 GiB  | 0.43 bit      | 98s                | 8963s             | 91x      |
| 505.mcf_r       | N/A           | 2.21e11   | 31.0 GiB   | 1.20 bit      | 168s               | 4800s             | 29x      |
| 520.omnetpp_r   | N/A           | 2.15e11   | 13.3 GiB   | 0.53 bit      | 135s               | 7289s             | 54x      |
| 523.xalancbmk_r | N/A           | 3.27e11   | 4.45 GiB   | 0.12 bit      | 112s               | 8883s             | 79x      |
| 525.x264_r      | pass 1        | 1.44e10   | 579 MiB    | 0.34 bit      | 14s                | 348s              | 25x      |
| 525.x264_r      | pass 2        | 4.42e10   | 2.30 GiB   | 0.45 bit      | 39s                | 1202s             | 31x      |
| 525.x264_r      | seek 500      | 4.78e10   | 2.77 GiB   | 0.50 bit      | 41s                | 1258s             | 31x      |
| 525.x264_r      | Total         | 1.06e11   | 5.64 GiB   | 0.46 bit      | 94s                | 2808s             | 30x      |
| 531.deepsjeng_r | N/A           | 2.74e11   | 31.6 GiB   | 0.99 bit      | 140s               | 8093s             | 58x      |
| 541.leela_r     | N/A           | 3.38e11   | 75.6 GiB   | 1.92 bit      | 224s               | 8894s             | 40x      |
| 548.exchange2_r | N/A           | 3.01e11   | 26.3 GiB   | 0.75 bit      | 88s                | 6753s             | 77x      |
| 557.xz_r        | cld           | 5.08e10   | 9.16 GiB   | 1.55 bit      | 60s                | 1252s             | 21x      |
| 557.xz_r        | cpu2006docs   | 1.84e11   | 7.80 GiB   | 0.36 bit      | 65s                | 3923s             | 60x      |
| 557.xz_r        | input         | 7.96e10   | 10.5 GiB   | 1.14 bit      | 55s                | 1842s             | 33x      |
| 557.xz_r        | Total         | 3.14e11   | 27.5 GiB   | 0.75 bit      | 180s               | 7017s             | 39x      |
| Total           | N/A           | 2.86e12   | 241 GiB    | 0.72 bit      | 1362s              | 77834s            | 57x      |

### Simulation

MPKI of different predictors:

| benchmark       | TAGE-SC-L 8KB | TAGE-SC-L 8KB Only TAGE | TAGE-SC-L 64KB | TAGE-SC-L 64KB Only TAGE | TAGE-Cookbook | Andre Seznec Unlimited |
|-----------------|---------------|-------------------------|----------------|--------------------------|---------------|------------------------|
| 500.perlbench_r | 1.0033        | 1.0464                  | 0.7209         | 0.7676                   | 0.7774        | 0.4191                 |
| 502.gcc_r       | 4.6880        | 4.8908                  | 3.3600         | 3.6323                   | 3.5698        | 1.5747                 |
| 505.mcf_r       | 13.0168       | 14.0096                 | 12.2277        | 13.3399                  | 12.3923       | 10.7859                |
| 520.omnetpp_r   | 4.0858        | 4.3132                  | 3.4897         | 4.1885                   | 3.8276        | 2.7857                 |
| 523.xalancbmk_r | 0.8520        | 0.9810                  | 0.6817         | 0.8793                   | 0.8664        | 0.2289                 |
| 525.x264_r      | 0.7596        | 0.7965                  | 0.5879         | 0.6571                   | 0.6332        | 0.4523                 |
| 531.deepsjeng_r | 4.5924        | 4.7104                  | 3.4531         | 3.8108                   | 3.5525        | 2.1531                 |
| 541.leela_r     | 11.7913       | 12.4307                 | 9.4221         | 10.1996                  | 9.6553        | 6.8911                 |
| 548.exchange2_r | 2.9609        | 3.6632                  | 1.2488         | 1.9521                   | 1.7593        | 0.3846                 |
| 557.xz_r        | 4.6811        | 5.0213                  | 4.0627         | 4.5943                   | 4.2971        | 3.1187                 |
| average         | 4.8431        | 5.1863                  | 3.9255         | 4.4021                   | 4.1331        | 2.8794                 |
| space           | 67349 bits    | 58165 bits              | 523355 bits    | 463917 bits              | 558273 bits   | N/A                    |

Difference between two builds (Run #1 without `-g`, Run #2 with `-g`):

| Executions | TAGE-SC-L 8KB | TAGE-SC-L 8KB Only TAGE | TAGE-SC-L 64KB | TAGE-SC-L 64KB Only TAGE | TAGE-Cookbook | Andre Seznec Unlimited |
|------------|---------------|-------------------------|----------------|--------------------------|---------------|------------------------|
| Run #1     | 4.8431        | 5.1863                  | 3.9255         | 4.4021                   | 4.1331        | 2.8794                 |
| Run #2     | 4.7859        | 5.1256                  | 3.8887         | 4.3604                   | 4.0908        | 2.8703                 |
| Diff       | -1.2%         | -1.2%                   | -0.9%          | -0.9%                    | -1.0%         | -0.3%                  |

#### SimPoint

Accuracy of SimPoint versus full simulation using TAGE-SC-L 8KB:

| Benchmark  | Sub-benchmark | Full MPKI | SimPoint MPKI | Diff  |
|------------|---------------|-----------|---------------|-------|
| 525.x264_r | pass 1        | 0.5162    | 0.5090        | -1.4% |

#### Summary

Simulation result using TAGE-SC-L-8KB:

| Benchmark       | CMPKI   | # Static H2P br. | Misp. due to H2P br. | Acc. of cond. br. | Acc. of cond. br. excl. H2P |
|-----------------|---------|------------------|----------------------|-------------------|-----------------------------|
| 500.perlbench_r | 0.9707  | 2                | 28.18 %              | 99.32 %           | 99.50 %                     |
| 502.gcc_r       | 4.3491  | 2                | 1.78 %               | 97.48 %           | 97.50 %                     |
| 505.mcf_r       | 13.0173 | 21               | 94.89 %              | 92.13 %           | 98.94 %                     |
| 520.omnetpp_r   | 4.1142  | 11               | 84.71 %              | 96.82 %           | 99.32 %                     |
| 523.xalancbmk_r | 0.8194  | 5                | 68.13 %              | 99.72 %           | 99.88 %                     |
| 525.x264_r      | 0.7438  | 0                | 0.00 %               | 98.27 %           | 98.27 %                     |
| 531.deepsjeng_r | 4.4695  | 7                | 15.78 %              | 95.13 %           | 95.34 %                     |
| 541.leela_r     | 11.7980 | 36               | 57.29 %              | 87.96 %           | 91.59 %                     |
| 548.exchange2_r | 2.9664  | 15               | 55.28 %              | 98.09 %           | 98.68 %                     |
| 557.xz_r        | 4.6105  | 20               | 77.36 %              | 96.66 %           | 97.60 %                     |
| Average         | 4.7859  | 11.9             | 48.34 %              | 96.16 %           | 97.66 %                     |

#### HotSpot

502.gcc_r:

| Br. PC     | Exec. count | Misp. count | Taken rate (%) | Image & offset                                 | Source location                       |
|------------|-------------|-------------|----------------|------------------------------------------------|---------------------------------------|
| 0x0041076f | 758050472   | 41851680    | 29.98          | spec2017/build-o3/502.gcc_r/502.gcc_r:0x1076f  | spec2017/src/502.gcc_r/bitmap.c:647   |
| 0x004109ba | 912253178   | 40571714    | 84.68          | spec2017/build-o3/502.gcc_r/502.gcc_r:0x109ba  | spec2017/src/502.gcc_r/bitmap.c:563   |
| 0x0062084b | 134080334   | 26047692    | 80.53          | spec2017/build-o3/502.gcc_r/502.gcc_r:0x22084b | spec2017/src/502.gcc_r/bitmap.h:513   |
| 0x0041072a | 796366208   | 26036142    | 90.59          | spec2017/build-o3/502.gcc_r/502.gcc_r:0x1072a  | spec2017/src/502.gcc_r/bitmap.c:563   |
| 0x00bb7986 | 138640124   | 24684806    | 73.17          | spec2017/build-o3/502.gcc_r/502.gcc_r:0x7b7986 | unknown                               |
| 0x0040fe6f | 118890012   | 22210178    | 67.87          | spec2017/build-o3/502.gcc_r/502.gcc_r:0xfe6f   | spec2017/src/502.gcc_r/bitmap.c:206   |
| 0x0041071f | 920352868   | 18887494    | 13.47          | spec2017/build-o3/502.gcc_r/502.gcc_r:0x1071f  | spec2017/src/502.gcc_r/bitmap.c:562   |
| 0x00412fa0 | 2206989420  | 18231914    | 91.50          | spec2017/build-o3/502.gcc_r/502.gcc_r:0x12fa0  | spec2017/src/502.gcc_r/bitmap.c:1586  |
| 0x00410d4c | 61356278    | 17924800    | 50.13          | spec2017/build-o3/502.gcc_r/502.gcc_r:0x10d4c  | spec2017/src/502.gcc_r/bitmap.c:800   |
| 0x0094dd5b | 133671554   | 17673724    | 68.54          | spec2017/build-o3/502.gcc_r/502.gcc_r:0x54dd5b | spec2017/src/502.gcc_r/rtlanal.c:1704 |

505.mcf_r:

| Br. PC     | Exec. count | Misp. count | Taken rate (%) | Image & offset                               | Source location                                    |
|------------|-------------|-------------|----------------|----------------------------------------------|----------------------------------------------------|
| 0x00404ca3 | 17182767712 | 4323792282  | 34.91          | spec2017/build-o3/505.mcf_r/505.mcf_r:0x4ca3 | spec2017/src/505.mcf_r/pbeampp.c:68                |
| 0x00404caa | 11184097644 | 1603404188  | 49.64          | spec2017/build-o3/505.mcf_r/505.mcf_r:0x4caa | spec2017/src/505.mcf_r/pbeampp.c:70                |
| 0x00404e93 | 9060683534  | 1212396570  | 82.42          | spec2017/build-o3/505.mcf_r/505.mcf_r:0x4e93 | spec2017/src/505.mcf_r/pbeampp.c:54                |
| 0x0040527f | 8104138224  | 963834232   | 57.58          | spec2017/build-o3/505.mcf_r/505.mcf_r:0x527f | spec2017/src/505.mcf_r/spec_qsort/spec_qsort.c:150 |
| 0x004052d0 | 7936514322  | 867269938   | 40.72          | spec2017/build-o3/505.mcf_r/505.mcf_r:0x52d0 | spec2017/src/505.mcf_r/spec_qsort/spec_qsort.c:158 |
| 0x00402ce4 | 3588402694  | 715522502   | 50.95          | spec2017/build-o3/505.mcf_r/505.mcf_r:0x2ce4 | spec2017/src/505.mcf_r/implicit.c:358              |
| 0x00405ade | 1512858748  | 442401614   | 35.94          | spec2017/build-o3/505.mcf_r/505.mcf_r:0x5ade | spec2017/src/505.mcf_r/spec_qsort/spec_qsort.c:128 |
| 0x00403289 | 3493403856  | 433868296   | 77.66          | spec2017/build-o3/505.mcf_r/505.mcf_r:0x3289 | spec2017/src/505.mcf_r/implicit.c:631              |
| 0x00405a4f | 652125742   | 250801564   | 56.87          | spec2017/build-o3/505.mcf_r/505.mcf_r:0x5a4f | spec2017/src/505.mcf_r/spec_qsort/spec_qsort.c:125 |
| 0x00405b16 | 838447912   | 229717114   | 66.45          | spec2017/build-o3/505.mcf_r/505.mcf_r:0x5b16 | spec2017/src/505.mcf_r/spec_qsort/spec_qsort.c:126 |

520.omnetpp_r:

| Br. PC     | Exec. count | Misp. count | Taken rate (%) | Image & offset                                         | Source location                                          |
|------------|-------------|-------------|----------------|--------------------------------------------------------|----------------------------------------------------------|
| 0x0049e67b | 5715311684  | 1414816740  | 62.61          | spec2017/build-o3/520.omnetpp_r/520.omnetpp_r:0x9e67b  | spec2017/src/520.omnetpp_r/simulator/cmessageheap.cc:54  |
| 0x0049e6f0 | 3578157590  | 686098610   | 63.85          | spec2017/build-o3/520.omnetpp_r/520.omnetpp_r:0x9e6f0  | spec2017/src/520.omnetpp_r/simulator/cmessageheap.cc:55  |
| 0x0049e712 | 1293469558  | 497728974   | 52.28          | spec2017/build-o3/520.omnetpp_r/520.omnetpp_r:0x9e712  | spec2017/src/520.omnetpp_r/simulator/cmessageheap.cc:57  |
| 0x0049e6a3 | 5596729652  | 278137710   | 5.58           | spec2017/build-o3/520.omnetpp_r/520.omnetpp_r:0x9e6a3  | spec2017/src/520.omnetpp_r/simulator/cmessageheap.cc:202 |
| 0x0049e42d | 1545159106  | 184101036   | 16.67          | spec2017/build-o3/520.omnetpp_r/520.omnetpp_r:0x9e42d  | spec2017/src/520.omnetpp_r/simulator/cmessageheap.cc:55  |
| 0x0049e68c | 5751430074  | 147362600   | 2.62           | spec2017/build-o3/520.omnetpp_r/520.omnetpp_r:0x9e68c  | spec2017/src/520.omnetpp_r/simulator/cmessageheap.cc:54  |
| 0x00593f9a | 694163770   | 143466354   | 45.22          | spec2017/build-o3/520.omnetpp_r/520.omnetpp_r:0x193f9a | unknown                                                  |
| 0x0057e847 | 446836178   | 110249372   | 57.93          | spec2017/build-o3/520.omnetpp_r/520.omnetpp_r:0x17e847 | spec2017/src/520.omnetpp_r/model/EtherMAC.cc:221         |
| 0x00593f8b | 743742800   | 102283724   | 48.87          | spec2017/build-o3/520.omnetpp_r/520.omnetpp_r:0x193f8b | unknown                                                  |
| 0x00593fbb | 380272512   | 70071752    | 49.87          | spec2017/build-o3/520.omnetpp_r/520.omnetpp_r:0x193fbb | unknown                                                  |

531.deepsjeng_r:

| Br. PC     | Exec. count | Misp. count | Taken rate (%) | Image & offset                                           | Source location                              |
|------------|-------------|-------------|----------------|----------------------------------------------------------|----------------------------------------------|
| 0x0040ea2a | 3868393570  | 310149938   | 86.86          | spec2017/build-o3/531.deepsjeng_r/531.deepsjeng_r:0xea2a | spec2017/src/531.deepsjeng_r/search.cpp:378  |
| 0x0040492a | 4534574380  | 277524188   | 43.63          | spec2017/build-o3/531.deepsjeng_r/531.deepsjeng_r:0x492a | spec2017/src/531.deepsjeng_r/bits.cpp:18     |
| 0x00404915 | 5384169610  | 251926726   | 52.53          | spec2017/build-o3/531.deepsjeng_r/531.deepsjeng_r:0x4915 | spec2017/src/531.deepsjeng_r/bits.cpp:18     |
| 0x0040e020 | 2121554720  | 205965594   | 11.42          | spec2017/build-o3/531.deepsjeng_r/531.deepsjeng_r:0xe020 | spec2017/src/531.deepsjeng_r/search.cpp:248  |
| 0x00401f0d | 879770816   | 185234652   | 35.65          | spec2017/build-o3/531.deepsjeng_r/531.deepsjeng_r:0x1f0d | spec2017/src/531.deepsjeng_r/attacks.cpp:138 |
| 0x00401f20 | 879772200   | 182065792   | 42.56          | spec2017/build-o3/531.deepsjeng_r/531.deepsjeng_r:0x1f20 | spec2017/src/531.deepsjeng_r/attacks.cpp:142 |
| 0x0040e96d | 1091159358  | 164747362   | 20.18          | spec2017/build-o3/531.deepsjeng_r/531.deepsjeng_r:0xe96d | spec2017/src/531.deepsjeng_r/search.cpp:284  |
| 0x00401f34 | 879772200   | 160910536   | 31.16          | spec2017/build-o3/531.deepsjeng_r/531.deepsjeng_r:0x1f34 | spec2017/src/531.deepsjeng_r/attacks.cpp:146 |
| 0x0040f68c | 683626096   | 134817432   | 18.36          | spec2017/build-o3/531.deepsjeng_r/531.deepsjeng_r:0xf68c | spec2017/src/531.deepsjeng_r/search.cpp:1068 |
| 0x00401f44 | 879772200   | 134314972   | 28.94          | spec2017/build-o3/531.deepsjeng_r/531.deepsjeng_r:0x1f44 | spec2017/src/531.deepsjeng_r/attacks.cpp:150 |

541.leela_r:

| Br. PC     | Exec. count | Misp. count | Taken rate (%) | Image & offset                                    | Source location                             |
|------------|-------------|-------------|----------------|---------------------------------------------------|---------------------------------------------|
| 0x0041e898 | 4831307334  | 1252859956  | 70.72          | spec2017/build-o3/541.leela_r/541.leela_r:0x1e898 | spec2017/src/541.leela_r/FastBoard.cpp:812  |
| 0x0042bdee | 7862705590  | 1197762890  | 19.09          | spec2017/build-o3/541.leela_r/541.leela_r:0x2bdee | spec2017/src/541.leela_r/FastState.cpp:177  |
| 0x0041f0bb | 4219345932  | 1044502130  | 36.12          | spec2017/build-o3/541.leela_r/541.leela_r:0x1f0bb | spec2017/src/541.leela_r/FastBoard.cpp:1275 |
| 0x0042b60d | 2059502010  | 1028215454  | 49.85          | spec2017/build-o3/541.leela_r/541.leela_r:0x2b60d | spec2017/src/541.leela_r/FastState.cpp:90   |
| 0x0041a777 | 3473679910  | 823392266   | 58.25          | spec2017/build-o3/541.leela_r/541.leela_r:0x1a777 | spec2017/src/541.leela_r/FastBoard.cpp:192  |
| 0x0041f0eb | 2479799140  | 753519180   | 67.98          | spec2017/build-o3/541.leela_r/541.leela_r:0x1f0eb | spec2017/src/541.leela_r/FastBoard.cpp:1220 |
| 0x0041edc2 | 1975026272  | 699081696   | 49.94          | spec2017/build-o3/541.leela_r/541.leela_r:0x1edc2 | spec2017/src/541.leela_r/FastBoard.cpp:1119 |
| 0x00424d6a | 3156694794  | 568532818   | 25.94          | spec2017/build-o3/541.leela_r/541.leela_r:0x24d6a | spec2017/src/541.leela_r/FastBoard.cpp:1172 |
| 0x0041f136 | 1810297996  | 456145606   | 75.86          | spec2017/build-o3/541.leela_r/541.leela_r:0x1f136 | spec2017/src/541.leela_r/FastBoard.cpp:1220 |
| 0x0042baa4 | 1314208508  | 434987028   | 62.49          | spec2017/build-o3/541.leela_r/541.leela_r:0x2baa4 | spec2017/src/541.leela_r/FastState.cpp:233  |

548.exchange2_r:

| Br. PC     | Exec. count | Misp. count | Taken rate (%) | Image & offset                                            | Source location                                 |
|------------|-------------|-------------|----------------|-----------------------------------------------------------|-------------------------------------------------|
| 0x0041b8dd | 13109357242 | 440610306   | 85.33          | spec2017/build-o3/548.exchange2_r/548.exchange2_r:0x1b8dd | spec2017/src/548.exchange2_r/exchange2.F90:1058 |
| 0x0041ba1d | 17306981236 | 386969326   | 91.73          | spec2017/build-o3/548.exchange2_r/548.exchange2_r:0x1ba1d | spec2017/src/548.exchange2_r/exchange2.F90:1080 |
| 0x0041b82f | 9597577206  | 317967916   | 84.66          | spec2017/build-o3/548.exchange2_r/548.exchange2_r:0x1b82f | spec2017/src/548.exchange2_r/exchange2.F90:1047 |
| 0x0041baa5 | 12880740254 | 238987768   | 95.52          | spec2017/build-o3/548.exchange2_r/548.exchange2_r:0x1baa5 | spec2017/src/548.exchange2_r/exchange2.F90:1091 |
| 0x0041cbde | 5560436222  | 230853082   | 89.11          | spec2017/build-o3/548.exchange2_r/548.exchange2_r:0x1cbde | spec2017/src/548.exchange2_r/exchange2.F90:1080 |
| 0x0041cb31 | 4604803178  | 178843790   | 86.58          | spec2017/build-o3/548.exchange2_r/548.exchange2_r:0x1cb31 | spec2017/src/548.exchange2_r/exchange2.F90:1069 |
| 0x0041b6c5 | 5517732010  | 170740948   | 80.67          | spec2017/build-o3/548.exchange2_r/548.exchange2_r:0x1b6c5 | spec2017/src/548.exchange2_r/exchange2.F90:1025 |
| 0x00430b77 | 2855371092  | 151213154   | 10.50          | spec2017/build-o3/548.exchange2_r/548.exchange2_r:0x30b77 | unknown                                         |
| 0x0041cc85 | 5447095468  | 131068256   | 94.84          | spec2017/build-o3/548.exchange2_r/548.exchange2_r:0x1cc85 | spec2017/src/548.exchange2_r/exchange2.F90:1091 |
| 0x00430c04 | 2625386316  | 122225606   | 95.30          | spec2017/build-o3/548.exchange2_r/548.exchange2_r:0x30c04 | unknown                                         |

557.xz_r:

| Br. PC     | Exec. count | Misp. count | Taken rate (%) | Image & offset                              | Source location                                                      |
|------------|-------------|-------------|----------------|---------------------------------------------|----------------------------------------------------------------------|
| 0x0040e4c5 | 23056940900 | 1350636190  | 11.01          | spec2017/build-o3/557.xz_r/557.xz_r:0xe4c5  | spec2017/src/557.xz_r/liblzma/lz/lz_encoder_mf.c:488                 |
| 0x0040e472 | 8925122938  | 951933576   | 78.87          | spec2017/build-o3/557.xz_r/557.xz_r:0xe472  | spec2017/src/557.xz_r/liblzma/lz/lz_encoder_mf.c:505                 |
| 0x0040e46e | 8926045620  | 749081010   | 28.46          | spec2017/build-o3/557.xz_r/557.xz_r:0xe46e  | spec2017/src/557.xz_r/liblzma/lz/lz_encoder_mf.c:486                 |
| 0x0040e6cd | 48589410416 | 701788306   | 2.66           | spec2017/build-o3/557.xz_r/557.xz_r:0xe6cd  | spec2017/src/557.xz_r/liblzma/lz/lz_encoder_mf.c:553                 |
| 0x00413e9e | 4341515970  | 566988822   | 23.02          | spec2017/build-o3/557.xz_r/557.xz_r:0x13e9e | spec2017/src/557.xz_r/liblzma/lzma/lzma_encoder_optimum_normal.c:752 |
| 0x00413e74 | 4341515970  | 423400868   | 79.14          | spec2017/build-o3/557.xz_r/557.xz_r:0x13e74 | spec2017/src/557.xz_r/liblzma/lzma/lzma_encoder_optimum_normal.c:744 |
| 0x0040fd7b | 4425548466  | 375046682   | 89.09          | spec2017/build-o3/557.xz_r/557.xz_r:0xfd7b  | spec2017/src/557.xz_r/liblzma/lz/lz_encoder_mf.c:716                 |
| 0x00414492 | 7252687226  | 345500206   | 94.75          | spec2017/build-o3/557.xz_r/557.xz_r:0x14492 | spec2017/src/557.xz_r/liblzma/lzma/lzma_encoder_optimum_normal.c:631 |
| 0x0040e52c | 2539179426  | 248644214   | 70.09          | spec2017/build-o3/557.xz_r/557.xz_r:0xe52c  | spec2017/src/557.xz_r/liblzma/lz/lz_encoder_mf.c:491                 |
| 0x0040e689 | 1895524512  | 247648466   | 52.88          | spec2017/build-o3/557.xz_r/557.xz_r:0xe689  | spec2017/src/557.xz_r/liblzma/lz/lz_encoder_mf.c:563                 |

## benchmarks/spec-int-2017-rate-o3-lto

- Platform: Intel i9-14900K
- Compiler: GCC 12.2.0
- Optimization Flags: -O3 -flto
- Tracer: Pin

### Tracing

Summary:

| Benchmark       | Sub-benchmark | Br. Exec. | Trace Size |
|-----------------|---------------|-----------|------------|
| 500.perlbench_r | checkspam     | 2.32e11   | 8.77 GiB   |
| 500.perlbench_r | diffmail      | 1.45e11   | 2.70 GiB   |
| 500.perlbench_r | splitmail     | 1.31e11   | 1.48 GiB   |
| 500.perlbench_r | Total         | 5.08e11   | 12.95 GiB  |
| 502.gcc_r       | gcc-pp -O3    | 4.27e10   | 3.20 GiB   |
| 502.gcc_r       | gcc-pp -O2    | 5.10e10   | 3.38 GiB   |
| 502.gcc_r       | gcc-smaller   | 5.31e10   | 2.78 GiB   |
| 502.gcc_r       | ref32 -O5     | 4.05e10   | 1.17 GiB   |
| 502.gcc_r       | ref32 -O3     | 4.58e10   | 1.45 GiB   |
| 502.gcc_r       | Total         | 2.33e11   | 11.98 GiB  |
| 505.mcf_r       | N/A           | 1.62e11   | 26.6 GiB   |
| 520.omnetpp_r   | N/A           | 1.97e11   | 13.2 GiB   |
| 523.xalancbmk_r | N/A           | 3.16e11   | 4.37 GiB   |
| 525.x264_r      | pass 1        | 1.43e10   | 575 MiB    |
| 525.x264_r      | pass 2        | 4.42e10   | 2.29 GiB   |
| 525.x264_r      | seek 500      | 4.77e10   | 2.76 GiB   |
| 525.x264_r      | Total         | 1.06e11   | 5.61 GiB   |
| 531.deepsjeng_r | N/A           | 2.13e11   | 29.1 GiB   |
| 541.leela_r     | N/A           | 2.61e11   | 72.3 GiB   |
| 548.exchange2_r | N/A           | 3.02e11   | 26.2 GiB   |
| 557.xz_r        | cld           | 5.07e10   | 9.12 GiB   |
| 557.xz_r        | cpu2006docs   | 1.84e11   | 7.78 GiB   |
| 557.xz_r        | input         | 7.94e10   | 10.5 GiB   |
| 557.xz_r        | Total         | 3.14e11   | 27.4 GiB   |
| Total           | N/A           | 2.61e12   | 230 GiB    |

### Simulation

MPKI of different predictors:

| benchmark       | TAGE-SC-L 8KB | TAGE-SC-L 8KB Only TAGE | TAGE-SC-L 64KB | TAGE-SC-L 64KB Only TAGE | TAGE-Cookbook | Andre Seznec Unlimited |
|-----------------|---------------|-------------------------|----------------|--------------------------|---------------|------------------------|
| 500.perlbench_r | 0.8881        | 0.9181                  | 0.6270         | 0.6714                   | 0.6953        | 0.3264                 |
| 502.gcc_r       | 4.5784        | 4.7348                  | 3.2394         | 3.4249                   | 3.4073        | 1.5709                 |
| 505.mcf_r       | 19.1913       | 20.6551                 | 17.8490        | 19.5186                  | 18.2302       | 15.7669                |
| 520.omnetpp_r   | 4.3221        | 4.5707                  | 3.6933         | 4.4184                   | 4.0436        | 2.9969                 |
| 523.xalancbmk_r | 0.9029        | 1.0411                  | 0.7328         | 0.9242                   | 0.9038        | 0.2751                 |
| 525.x264_r      | 0.7673        | 0.8050                  | 0.5915         | 0.6604                   | 0.6353        | 0.4534                 |
| 531.deepsjeng_r | 4.7837        | 4.9058                  | 3.6346         | 4.0051                   | 3.6892        | 2.2254                 |
| 541.leela_r     | 13.9617       | 14.4772                 | 10.9097        | 11.7822                  | 11.0579       | 7.8981                 |
| 548.exchange2_r | 2.9139        | 3.5762                  | 1.1552         | 1.7467                   | 1.6100        | 0.3397                 |
| 557.xz_r        | 4.7128        | 5.0792                  | 4.0751         | 4.6535                   | 4.3472        | 3.0562                 |
| average         | 5.7022        | 6.0763                  | 4.6507         | 5.1805                   | 4.8620        | 3.4909                 |
| space           | 67349 bits    | 58165 bits              | 523355 bits    | 463917 bits              | 558273 bits   | N/A                    |

`-O3` vs `-O3 -flto`:

| Executions | TAGE-SC-L 8KB | TAGE-SC-L 8KB Only TAGE | TAGE-SC-L 64KB | TAGE-Cookbook | Andre Seznec Unlimited |
|------------|---------------|-------------------------|----------------|---------------|------------------------|
| O3         | 4.7859        | 5.1256                  | 3.8887         | 4.0908        | 2.8703                 |
| O3+LTO     | 5.7022        | 6.0763                  | 4.6507         | 4.8620        | 3.4909                 |
| Diff       | +19.1%        | +18.5%                  | +19.6%         | +18.9%        | +21.6%                 |
