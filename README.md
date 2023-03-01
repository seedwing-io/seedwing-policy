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

All the policies can be centrally managed in a server, or built in as part of a custom application.

See the [documentation](https://docs.seedwing.io/docs/index.html) for more information.

### Other use cases

Seedwing Policy is primarily concerned with software supply chain, but may be used in other contexts as well such as authorization policies for Apache Kafka.

## Usage

To use the policy engine, download the latest released `swio` binary for your platform. 

To evaluate policies:

```ignore
swio -p <policy dir> eval --name mypolicy::pattern --input input.json
```

To run the HTTP server (point your browser to the http://localhost:8080 to view the console):

```ignore
swio -p <policy dir> serve
```

To benchmark policies:

```ignore
swio -p <policy dir> bench --name mypolicy::pattern --input input.json --count 1000
```

## Minimum supported Rust version (MSRV)

Seedwing Policy is guaranteed to compile on the latest stable Rust version at the time of release. It might compile with older versions.

## Development setup

See [CONTRIBUTING.md](CONTRIBUTING.md#setup)

## License

Apache License, Version 2.0 ([LICENSE](LICENSE))
