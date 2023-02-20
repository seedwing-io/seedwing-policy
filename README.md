# Seedwing Policy

[![CI](https://github.com/seedwing-io/seedwing-policy/workflows/CI/badge.svg)](https://github.com/seedwing-io/seedwing-policy/actions?query=workflow%3A%22CI%22)
[![crates.io](https://img.shields.io/crates/v/seedwing-policy-engine.svg)](https://crates.io/crates/seedwing-policy-engine)
[![docs.rs](https://docs.rs/seedwing-policy-engine/badge.svg)](https://docs.rs/seedwing-policy-engine)

A functional type system for implementing policy inspection, audit and enforcement.

Seedwing Policy consists of several components that may be combined or used standalone as part of a secure software supply chain:

* *Dogma* - a policy description language.
* *Engine* - a policy evaluation engine.
* *Server* - an HTTP server and API for evaluating policies.

With Seedwing Policy, you can:

* Validate, destructure and inspect payload according to standards like CycloneDX, SPDX, OpenVEX, PEM and more.
* Check for permitted licenses according to organization policy.
* Check for trusted signatures against [Sigstore](https://sigstore.dev).
* Check SBOM dependencies for vulnerabilities against [OSV](https://osv.dev).

All of the policies can be centrally managed in a server, or built in as part of a custom application.

### Other use cases

Seedwing Policy is primarily concerned with software supply chain, but may be used in other contexts as well such as authorization policies for Apacke Kafka.

## Minimum supported Rust version (MSRV)

Seedwing Policy is guaranteed to compile on the latest stable Rust version at the time of release. It might compile with older versions.

## Development setup

You will need the following tools and components:

* **Rust**: For the overall project
* **Node.js & Yarn**: For the web bits in the server
* **Podman or Docker**: For building the container image
* **Trunk**: For building the dedicated web frontend

Building seedwing-policy requires `nodejs` and `yarnpkg`. You can follow installation instructions [here](https://developer.fedoraproject.org/tech/languages/nodejs/nodejs.html) or run the commands appropriate for your development environment.

`trunk` can be installed by executing `cargo install trunk`. It additionally requires `wasm-bindgen` and `dart-sass`, but will automatically install those tools if they are missing. Also see: https://trunkrs.dev/#install

### Fedora

```shell
dnf install nodejs yarnpkg
```

### Mac OS

```shell
brew install yarn node
```

## License

Apache License, Version 2.0 ([LICENSE](LICENSE))
