Machine: Intel(R) Core(TM) Ultra 9 285H, 16 threads, 31 GB, Windows_NT 10.0.26300

rustc: rustc 1.100.0 (0ef1bc386 2026-11-09)  
pud: pud 0.1.0  
moon: moon 0.1.20261119 (738607b 2026-11-19)  
moonc: v0.10.22+124d61d05 (2026-11-19)  
node: v24.15.0

## Kernel time (ms, median, measured inside the program)

| bench | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| fib | 69.4 | 101 (1.5x) | 93 (1.3x) | 105 (1.5x) | 157 (2.3x) |
| sieve | 19.8 | 23 (1.2x) | 48 (2.4x) | 75 (3.8x) | 49 (2.5x) |
| nbody | 132.8 | 176 (1.3x) | 179 (1.3x) | 179 (1.3x) | 198 (1.5x) |
| trees | 3935.5 | 2022 (0.5x) | 525 (0.1x) | 573 (0.1x) | 1024 (0.3x) |
| mandel | 134.7 | 138 (1.0x) | 335 (2.5x) | 346 (2.6x) | 196 (1.5x) |
| hashmap | 38.5 | 279 (7.3x) | 264 (6.9x) | 73 (1.9x) | 110 (2.9x) |
| strings | 14.1 | 115 (8.2x) | 63 (4.5x) | 92 (6.5x) | 95 (6.7x) |
| sort | 14.3 | 73 (5.1x) | 79 (5.5x) | 141 (9.8x) | 141 (9.9x) |
| iter | 7.0 | 59 (8.5x) | 58 (8.3x) | 45 (6.4x) | 381 (54.8x) |
| dyn | 40.4 | 22 (0.5x) | 75 (1.9x) | 105 (2.6x) | 174 (4.3x) |

|  | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| geomean vs Rust | 1.00x | 2.10x | 2.28x | 2.38x | 3.27x |

## Spread of kernel time across runs (max-min)/median, %

| bench | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| fib | 4 | 7 | 2 | 5 | 3 |
| sieve | 1 | 4 | 4 | 5 | 7 |
| nbody | 4 | 3 | 4 | 4 | 2 |
| trees | 6 | 1 | 2 | 1 | 4 |
| mandel | 3 | 4 | 1 | 5 | 1 |
| hashmap | 9 | 10 | 5 | 14 | 9 |
| strings | 14 | 7 | 2 | 5 | 5 |
| sort | 8 | 5 | 3 | 4 | 5 |
| iter | 1 | 7 | 5 | 4 | 4 |
| dyn | 4 | 14 | 4 | 2 | 2 |

## End-to-end wall time (ms, median, includes start-up; Puddi `pud run` also compiles)

| bench | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| hello | 12 | 15 | 36 | 129 | 110 |
| fib | 83 | 120 | 130 | 228 | 267 |
| sieve | 32 | 39 | 88 | 199 | 161 |
| nbody | 145 | 192 | 216 | 302 | 310 |
| trees | 3973 | 2039 | 575 | 715 | 1140 |
| mandel | 147 | 155 | 373 | 469 | 307 |
| hashmap | 53 | 297 | 306 | 204 | 223 |
| strings | 27 | 132 | 103 | 219 | 209 |
| sort | 27 | 88 | 118 | 267 | 253 |
| iter | 20 | 77 | 96 | 170 | 492 |
| dyn | 53 | 39 | 114 | 228 | 283 |

## CPU time of the whole process tree (ms, median)

| bench | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| hello | 0 | 16 | 16 | 94 | 94 |
| fib | 78 | 109 | 125 | 203 | 266 |
| sieve | 31 | 31 | 78 | 188 | 141 |
| nbody | 141 | 188 | 203 | 281 | 281 |
| trees | 3953 | 2016 | 766 | 828 | 1141 |
| mandel | 141 | 141 | 344 | 453 | 281 |
| hashmap | 47 | 281 | 344 | 203 | 203 |
| strings | 31 | 125 | 125 | 234 | 188 |
| sort | 16 | 78 | 109 | 266 | 234 |
| iter | 16 | 78 | 78 | 156 | 469 |
| dyn | 47 | 31 | 94 | 219 | 266 |

## Peak committed memory of the process tree (MB, median)

| bench | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| hello | 1.0 | 1.6 | 6.4 | 24.1 | 38.0 |
| fib | 1.0 | 1.6 | 6.5 | 24.4 | 38.2 |
| sieve | 10.5 | 11.2 | 16.2 | 36.4 | 56.6 |
| nbody | 1.0 | 1.6 | 6.9 | 25.5 | 37.3 |
| trees | 70.1 | 36.4 | 196.5 | 214.1 | 150.4 |
| mandel | 1.0 | 1.6 | 6.6 | 26.6 | 37.4 |
| hashmap | 15.7 | 48.4 | 77.4 | 134.8 | 150.0 |
| strings | 8.0 | 16.2 | 56.8 | 112.2 | 86.1 |
| sort | 8.6 | 5.4 | 15.5 | 37.2 | 53.6 |
| iter | 1.0 | 1.6 | 7.1 | 24.9 | 37.6 |
| dyn | 1.0 | 1.6 | 7.9 | 26.3 | 37.7 |

## Build time (ms) and output size (KB)

| bench | rustc -O3 | Rust exe | pud build | Puddi wasm | MoonBit wasm-gc | MoonBit native |
|---|---|---|---|---|---|---|
| hello | 246 | 128 | 104 | 293 | 0.3 | 172 |
| fib | 259 | 154 | 110 | 294 | 3.9 | 178 |
| sieve | 282 | 155 | 104 | 295 | 4.0 | 178 |
| nbody | 317 | 161 | 102 | 299 | 12.6 | 189 |
| trees | 270 | 156 | 106 | 295 | 4.1 | 178 |
| mandel | 255 | 154 | 104 | 294 | 4.1 | 178 |
| hashmap | 363 | 158 | 112 | 295 | 7.4 | 181 |
| strings | 409 | 160 | 108 | 300 | 9.3 | 186 |
| sort | 369 | 161 | 114 | 297 | 7.0 | 182 |
| iter | 274 | 155 | 106 | 297 | 6.6 | 180 |
| dyn | 286 | 156 | 110 | 296 | 12.0 | 188 |

MoonBit builds all 11 programs as one module: wasm-gc 320 ms, native 1349 ms (clean, release).

Puddi output files for fib: fib.core.wasm 124.0 KB, fib.wasm 143.9 KB, puddi.mjs 25.7 KB

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

