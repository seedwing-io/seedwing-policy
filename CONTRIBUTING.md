# Contributing

Thank you for your interest in the project and for considering contributing.

This guide should help you get started: creating a build and test environment, as well as contributing your work.

All contributions are welcome! While this guide will focus on contributing code, we would also encourage you to contribute by reporting issues, providing feedback, suggesting new ideas. Or just by saying "hi" in the chat.

## Certificate of Origin

By contributing to this project you agree to the Developer Certificate of
Origin (DCO). This document was created by the Linux Kernel community and is a
simple statement that you, as a contributor, have the legal right to make the
contribution. See the [DCO](DCO) file for details.

## Before you start

Before you start working on a fix or new feature, we would recommend to reach out to us and tell us about it. Maybe
we already have this in our heads (and forgot to create an issue for it), or maybe we have an alternative already.

In any case, it is always good to create an issue, or join the chat and tell us about your issues or plans. We will
definitely try to help you.

## Developing

If you prefer to manually set up your host OS, you will need the following tools and components:

* **Rust**: For the overall project
* **Node.js & npm**: For the web bits in the server
* **Podman or Docker**: For building the container image
* **Trunk**: For building the web frontend

Building seedwing-policy requires `nodejs` and `npm`. You can follow installation instructions [here](https://developer.fedoraproject.org/tech/languages/nodejs/nodejs.html) or run the commands appropriate for your development environment.

`trunk` can be installed by executing `cargo install trunk`. It additionally requires `wasm-bindgen` and `dart-sass`, but will automatically install those tools if they are missing. Also see: https://trunkrs.dev/#install

**NOTE:** Trunk will re-use existing tooling when found on the local system. However, that tooling must be compatible
with the trunk toolchain. If it is not, it may lead to a failed build. See [trunk tooling](#trunk-tooling) for
setup instructions.

For more information on developing for the frontend, see: [seedwing-policy-frontend/](frontend/).

### Fedora

```shell
dnf install nodejs
```

### Mac OS

```shell
brew install node
```

### Trunk tooling

**NOTE:** In addition of installing these tools, please also ensure they can be located. You might need to add
them to the `$PATH` of your system and ensure there is no overlap wither other commands, having the same name.

```shell
npm install -g sass@1.58.3 && sass --version
```

## Toolbox

An alternative to install the dependencies manually is to use our [development container image](https://github.com/orgs/seedwing-io/packages/container/package/seedwing-policy-devcontainer).
You can set this up using the [toolbox](https://containertoolbx.org/) or the [devcontainer VSCode plugin](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers).
A benefit is that every dependency you need is shipped within the container, so no more packages to install and wondering where they're from when the next 
distribution upgrade fails.

Make sure to clone the git repo, still!

```shell
toolbox create seedwing --image ghcr.io/seedwing/seedwing-policy-devcontainer
toolbox enter seedwing
```

### VSCode Devcontainer
If you simply have the extension installed in VSCode it shoul pick up the `.devcontainer.json` file we provide in the repo. 
Let us know if you encounter any problems.

## Contributing your work

Thank you for reading the document up to this point and for taking the next step.

### Pre-flight check

Before creating a pull-request (PR), you should do some pre-flight checks, which the CI will run later on anyway.
Running locally will give you quicker results, and safe us a bit of time and CI resources.

Here is what you should do:

* Format the code with `rustfmt`. You can check using:

  ```shell
  cargo fmt --check
  cargo fmt --check --manifest-path frontend/Cargo.toml
  ```

* Ensure that the code builds. You can check using:

  ```shell
  cargo build
  ```

### Creating a PR

Nothing fancy, just a normal PR on GitHub.
