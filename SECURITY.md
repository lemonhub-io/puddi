# Security policy

## Supported versions

Puddi is in its 0.x series. Security fixes are made on `main` and released in the next version; only the latest release is supported.

## What counts as a security issue

- A Puddi program that escapes the WebAssembly sandbox of `pud run`, or reaches host resources (files, network, environment) that were not granted to it.
- Generated code that breaks memory or type safety, for example by skipping a bounds or overflow check.
- A dependency resolution or lock file problem that lets a package be replaced with different code.
- Problems in the install scripts or release archives, such as a missing or bypassable checksum check.

Ordinary crashes, panics and miscompilations that do not cross a trust boundary are bugs: please open a normal [issue](https://github.com/puddi-lang/puddi/issues).

## Reporting a vulnerability

Do not open a public issue. Report it privately through **Report a vulnerability** on the repository's [Security tab](https://github.com/puddi-lang/puddi/security/advisories/new). Include the affected version, the steps to reproduce and the impact you see.

This is a small volunteer project, so responses are best effort. We aim to acknowledge a report within a week, agree on a fix and a disclosure date with you, and credit you in the advisory unless you prefer otherwise.
