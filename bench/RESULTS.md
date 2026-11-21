Machine: Intel(R) Core(TM) Ultra 9 285H, 16 threads, 31 GB, Windows_NT 10.0.26300

rustc: rustc 1.100.0 (0ef1bc386 2026-11-09)  
pud: pud 0.1.0  
moon: moon 0.1.20261119 (738607b 2026-11-19)  
moonc: v0.10.22+124d61d05 (2026-11-19)  
node: v24.15.0

## Kernel time (ms, median, measured inside the program)

| bench | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| fib | 69.4 | 101 (1.5x) | 93 (1.3x) | 88 (1.3x) | 85 (1.2x) |
| sieve | 19.8 | 23 (1.2x) | 48 (2.4x) | 43 (2.2x) | 48 (2.4x) |
| nbody | 132.8 | 176 (1.3x) | 179 (1.3x) | 190 (1.4x) | 216 (1.6x) |
| trees | 3935.5 | 2022 (0.5x) | 525 (0.1x) | 572 (0.1x) | 957 (0.2x) |
| mandel | 134.7 | 138 (1.0x) | 335 (2.5x) | 139 (1.0x) | 197 (1.5x) |
| hashmap | 38.5 | 279 (7.3x) | 264 (6.9x) | 48 (1.3x) | 75 (2.0x) |
| strings | 14.1 | 115 (8.2x) | 63 (4.5x) | 77 (5.5x) | 89 (6.3x) |
| sort | 14.3 | 73 (5.1x) | 79 (5.5x) | 21 (1.4x) | 41 (2.8x) |
| iter | 7.0 | 59 (8.5x) | 58 (8.3x) | 10 (1.4x) | 11 (1.6x) |
| dyn | 40.4 | 22 (0.5x) | 75 (1.9x) | 49 (1.2x) | 184 (4.5x) |

|  | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| geomean vs Rust | 1.00x | 2.10x | 2.28x | 1.26x | 1.84x |

## Spread of kernel time across runs (max-min)/median, %

| bench | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| fib | 4 | 7 | 2 | 3 | 2 |
| sieve | 1 | 4 | 4 | 6 | 4 |
| nbody | 4 | 3 | 4 | 2 | 2 |
| trees | 6 | 1 | 2 | 1 | 2 |
| mandel | 3 | 4 | 1 | 2 | 2 |
| hashmap | 9 | 10 | 5 | 7 | 19 |
| strings | 14 | 7 | 2 | 6 | 4 |
| sort | 8 | 5 | 3 | 5 | 11 |
| iter | 1 | 7 | 5 | 11 | 4 |
| dyn | 4 | 14 | 4 | 4 | 3 |

## End-to-end wall time (ms, median, includes start-up; Puddi `pud run` also compiles)

| bench | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| hello | 12 | 15 | 36 | 123 | 75 |
| fib | 83 | 120 | 130 | 218 | 158 |
| sieve | 32 | 39 | 88 | 170 | 123 |
| nbody | 145 | 192 | 216 | 322 | 289 |
| trees | 3973 | 2039 | 575 | 714 | 1036 |
| mandel | 147 | 155 | 373 | 269 | 271 |
| hashmap | 53 | 297 | 306 | 187 | 150 |
| strings | 27 | 132 | 103 | 213 | 166 |
| sort | 27 | 88 | 118 | 153 | 115 |
| iter | 20 | 77 | 96 | 139 | 85 |
| dyn | 53 | 39 | 114 | 182 | 258 |

## CPU time of the whole process tree (ms, median)

| bench | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| hello | 0 | 16 | 16 | 125 | 78 |
| fib | 78 | 109 | 125 | 219 | 172 |
| sieve | 31 | 31 | 78 | 156 | 125 |
| nbody | 141 | 188 | 203 | 313 | 281 |
| trees | 3953 | 2016 | 766 | 906 | 1031 |
| mandel | 141 | 141 | 344 | 266 | 281 |
| hashmap | 47 | 281 | 344 | 156 | 141 |
| strings | 31 | 125 | 125 | 219 | 172 |
| sort | 16 | 78 | 109 | 141 | 125 |
| iter | 16 | 78 | 78 | 125 | 78 |
| dyn | 47 | 31 | 94 | 156 | 266 |

## Peak committed memory of the process tree (MB, median)

| bench | rust | moonbit-native | moonbit-wasm-gc | puddi-node | puddi-wasmtime |
|---|---|---|---|---|---|
| hello | 1.0 | 1.6 | 6.4 | 26.1 | 52.1 |
| fib | 1.0 | 1.6 | 6.5 | 27.1 | 42.0 |
| sieve | 10.5 | 11.2 | 16.2 | 37.5 | 59.6 |
| nbody | 1.0 | 1.6 | 6.9 | 28.3 | 41.0 |
| trees | 70.1 | 36.4 | 196.5 | 214.1 | 153.8 |
| mandel | 1.0 | 1.6 | 6.6 | 27.7 | 39.9 |
| hashmap | 15.7 | 48.4 | 77.4 | 105.5 | 153.7 |
| strings | 8.0 | 16.2 | 56.8 | 109.1 | 89.5 |
| sort | 8.6 | 5.4 | 15.5 | 36.8 | 40.9 |
| iter | 1.0 | 1.6 | 7.1 | 27.9 | 40.8 |
| dyn | 1.0 | 1.6 | 7.9 | 28.4 | 41.3 |

## Build time (ms) and output size (KB)

| bench | rustc -O3 | Rust exe | pud build | Puddi wasm | MoonBit wasm-gc | MoonBit native |
|---|---|---|---|---|---|---|
| hello | 246 | 128 | 75 | 229 | 0.3 | 172 |
| fib | 259 | 154 | 71 | 230 | 3.9 | 178 |
| sieve | 282 | 155 | 72 | 232 | 4.0 | 178 |
| nbody | 317 | 161 | 72 | 236 | 12.6 | 189 |
| trees | 270 | 156 | 73 | 238 | 4.1 | 178 |
| mandel | 255 | 154 | 72 | 231 | 4.1 | 178 |
| hashmap | 363 | 158 | 72 | 232 | 7.4 | 181 |
| strings | 409 | 160 | 75 | 238 | 9.3 | 186 |
| sort | 369 | 161 | 71 | 236 | 7.0 | 182 |
| iter | 274 | 155 | 73 | 237 | 6.6 | 180 |
| dyn | 286 | 156 | 73 | 233 | 12.0 | 188 |

MoonBit builds all 11 programs as one module: wasm-gc 320 ms, native 1349 ms (clean, release).

Puddi output files for fib: fib.core.wasm 95.7 KB, fib.wasm 108.7 KB, puddi.mjs 25.7 KB

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

