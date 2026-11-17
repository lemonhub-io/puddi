# WASI WIT definitions

The files in this directory are the WASI 0.3.2 interface definitions from
https://github.com/WebAssembly/WASI, bundled unmodified so that `pud` can build
WASI 0.3 components offline, and the `wasi:logging` interface from
https://github.com/WebAssembly/wasi-logging.

They are licensed under the Apache License 2.0 with LLVM Exception
(`Apache-2.0 WITH LLVM-exception`); copyright the WASI contributors. The Apache
License 2.0 text is in `LICENSE-APACHE` at the root of this repository. The LLVM exception reads:

    ---- LLVM Exceptions to the Apache 2.0 License ----

    As an exception, if, as a result of your compiling your source code, portions
    of this Software are embedded into an Object form of such source code, you
    may redistribute such embedded portions in such Object form without complying
    with the conditions of Sections 4(a), 4(b) and 4(d) of the License.

    In addition, if you combine or link compiled forms of this Software with
    software that is licensed under the GPLv2 ("Combined Software") and if a
    court of competent jurisdiction determines that the patent provision (Section
    3), the indemnity provision (Section 9) or other Section of the License
    conflicts with the conditions of the GPLv2, you may retroactively and
    prospectively choose to deem waived or otherwise exclude such Section(s) of
    the License, but only those Section(s) which are inconsistent with the
    conditions of the GPLv2.
