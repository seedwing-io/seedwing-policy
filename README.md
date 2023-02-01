# Seedwing Policy

[![CI](https://github.com/seedwing-io/seedwing-policy/workflows/CI/badge.svg)](https://github.com/seedwing-io/seedwing-policy/actions?query=workflow%3A%22CI%22)
[![crates.io](https://img.shields.io/crates/v/seedwing-policy-engine.svg)](https://crates.io/crates/seedwing-policy-engine)
[![docs.rs](https://docs.rs/seedwing-policy-engine/badge.svg)](https://docs.rs/seedwing-policy-engine)

A functional type system for implementing policy inspection, audit and enforcement.

## Minimum supported Rust version (MSRV)

Seedwing Policy is guaranteed to compile on the latest stable Rust version at the time of release. It might compile with older versions.

## Development setup

You will need the following tools and components:

* **Rust**: For the overall project
* **Node.js & Yarn**: For the web bits in the server
* **Podman or Docker**: For building the container image

### Fedora

Building seedwing-policy requires `nodejs` and `yarnpkg`. You can follow installation instructions [here](https://developer.fedoraproject.org/tech/languages/nodejs/nodejs.html), or run:

```shell
dnf install nodejs yarnpkg
```

## License

Apache License, Version 2.0 ([LICENSE](LICENSE))
