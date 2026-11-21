Machine: Intel(R) Core(TM) Ultra 9 285H, 16 threads, 31 GB, Windows_NT 10.0.26300

rustc: rustc 1.100.0 (0ef1bc386 2026-11-09)  
pud: pud 0.1.0  
moon: moon 0.1.20261119 (738607b 2026-11-19)  
moonc: v0.10.22+124d61d05 (2026-11-19)  
node: v24.15.0

## Kernel time (ms, median, measured inside the program)

| bench | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| fib | 69.4 | 101 (1.5x) | 93 (1.3x) | 90 (1.3x) | 87 (1.3x) |
| sieve | 19.8 | 23 (1.2x) | 48 (2.4x) | 44 (2.2x) | 48 (2.4x) |
| nbody | 132.8 | 176 (1.3x) | 179 (1.3x) | 191 (1.4x) | 215 (1.6x) |
| trees | 3935.5 | 2022 (0.5x) | 525 (0.1x) | 565 (0.1x) | 946 (0.2x) |
| mandel | 134.7 | 138 (1.0x) | 335 (2.5x) | 137 (1.0x) | 197 (1.5x) |
| hashmap | 38.5 | 279 (7.3x) | 264 (6.9x) | 46 (1.2x) | 66 (1.7x) |
| strings | 14.1 | 115 (8.2x) | 63 (4.5x) | 74 (5.3x) | 92 (6.6x) |
| sort | 14.3 | 73 (5.1x) | 79 (5.5x) | 21 (1.4x) | 43 (3.0x) |
| iter | 7.0 | 59 (8.5x) | 58 (8.3x) | 10 (1.4x) | 12 (1.7x) |
| dyn | 40.4 | 22 (0.5x) | 75 (1.9x) | 52 (1.3x) | 187 (4.6x) |

|  | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| geomean vs Rust | 1.00x | 2.10x | 2.28x | 1.25x | 1.85x |

## Spread of kernel time across runs (max-min)/median, %

| bench | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| fib | 4 | 7 | 2 | 5 | 2 |
| sieve | 1 | 4 | 4 | 7 | 5 |
| nbody | 4 | 3 | 4 | 7 | 4 |
| trees | 6 | 1 | 2 | 4 | 4 |
| mandel | 3 | 4 | 1 | 4 | 3 |
| hashmap | 9 | 10 | 5 | 2 | 14 |
| strings | 14 | 7 | 2 | 11 | 2 |
| sort | 8 | 5 | 3 | 9 | 5 |
| iter | 1 | 7 | 5 | 9 | 16 |
| dyn | 4 | 14 | 4 | 6 | 3 |

## End-to-end wall time (ms, median, includes start-up; Puddi `pud run` also compiles)

| bench | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| hello | 12 | 15 | 36 | 116 | 25 |
| fib | 83 | 120 | 130 | 214 | 113 |
| sieve | 32 | 39 | 88 | 170 | 75 |
| nbody | 145 | 192 | 216 | 320 | 244 |
| trees | 3973 | 2039 | 575 | 697 | 976 |
| mandel | 147 | 155 | 373 | 268 | 225 |
| hashmap | 53 | 297 | 306 | 177 | 96 |
| strings | 27 | 132 | 103 | 206 | 121 |
| sort | 27 | 88 | 118 | 148 | 72 |
| iter | 20 | 77 | 96 | 137 | 39 |
| dyn | 53 | 39 | 114 | 181 | 215 |

## CPU time of the whole process tree (ms, median)

| bench | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| hello | 0 | 16 | 16 | 109 | 16 |
| fib | 78 | 109 | 125 | 219 | 94 |
| sieve | 31 | 31 | 78 | 188 | 63 |
| nbody | 141 | 188 | 203 | 313 | 234 |
| trees | 3953 | 2016 | 766 | 781 | 953 |
| mandel | 141 | 141 | 344 | 266 | 203 |
| hashmap | 47 | 281 | 344 | 156 | 78 |
| strings | 31 | 125 | 125 | 219 | 94 |
| sort | 16 | 78 | 109 | 141 | 47 |
| iter | 16 | 78 | 78 | 141 | 16 |
| dyn | 47 | 31 | 94 | 172 | 188 |

## Peak committed memory of the process tree (MB, median)

| bench | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| hello | 1.0 | 1.6 | 6.4 | 25.9 | 34.0 |
| fib | 1.0 | 1.6 | 6.5 | 26.1 | 34.0 |
| sieve | 10.5 | 11.2 | 16.2 | 36.5 | 53.2 |
| nbody | 1.0 | 1.6 | 6.9 | 28.2 | 34.1 |
| trees | 70.1 | 36.4 | 196.5 | 213.3 | 146.3 |
| mandel | 1.0 | 1.6 | 6.6 | 28.1 | 34.0 |
| hashmap | 15.7 | 48.4 | 77.4 | 105.3 | 146.2 |
| strings | 8.0 | 16.2 | 56.8 | 109.6 | 82.2 |
| sort | 8.6 | 5.4 | 15.5 | 37.2 | 34.0 |
| iter | 1.0 | 1.6 | 7.1 | 27.6 | 34.0 |
| dyn | 1.0 | 1.6 | 7.9 | 27.9 | 34.0 |

## Build time (ms) and output size (KB)

| bench | rustc -O3 | Rust exe | pud build | Puddi wasm | MoonBit wasm-gc | MoonBit native |
|---|---|---|---|---|---|---|
| hello | 246 | 128 | 67 | 174 | 0.3 | 172 |
| fib | 259 | 154 | 60 | 175 | 3.9 | 178 |
| sieve | 282 | 155 | 58 | 176 | 4.0 | 178 |
| nbody | 317 | 161 | 60 | 179 | 12.6 | 189 |
| trees | 270 | 156 | 59 | 181 | 4.1 | 178 |
| mandel | 255 | 154 | 64 | 175 | 4.1 | 178 |
| hashmap | 363 | 158 | 59 | 176 | 7.4 | 181 |
| strings | 409 | 160 | 62 | 181 | 9.3 | 186 |
| sort | 369 | 161 | 63 | 179 | 7.0 | 182 |
| iter | 274 | 155 | 65 | 179 | 6.6 | 180 |
| dyn | 286 | 156 | 60 | 177 | 12.0 | 188 |

MoonBit builds all 11 programs as one module: wasm-gc 320 ms, native 1349 ms (clean, release).

Puddi output files for fib: fib.core.wasm 68.1 KB, fib.wasm 81.1 KB, puddi.mjs 25.7 KB

## Checksums

| bench | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| fib | 39088169 | 39088169 | 39088169 | 39088169 | 39088169 |
| sieve | 664579 | 664579 | 664579 | 664579 | 664579 |
| nbody | -169075164/-169083134 | -169075164/-169083134 | -169075164/-169083134 | -169075164.0/-169083134.0 | -169075164.0/-169083134.0 |
| trees | 68332206 | 68332206 | 68332206 | 68332206 | 68332206 |
| mandel | 1589711 | 1589711 | 1589711 | 1589711 | 1589711 |
| hashmap | 787411/393666/213066271130 | 787411/393666/213066271130 | 787411/393666/213066271130 | 787411/393666/213066271130 | 787411/393666/213066271130 |
| strings | 2889000/5000/100 | 2889000/5000/100 | 2889000/5000/100 | 2889000/5000/100 | 2889000/5000/100 |
| sort | true/1015/1074273426/2147482846 | true/1015/1074273426/2147482846 | true/1015/1074273426/2147482846 | true/1015/1074273426/2147482846 | true/1015/1074273426/2147482846 |
| iter | 1668330000 | 1668330000 | 1668330000 | 1668330000 | 1668330000 |
| dyn | 295500000 | 295500000 | 295500000 | 295500000.0 | 295500000.0 |

