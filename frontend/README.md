# Seedwing Policy Server Frontend

This is the frontend used by the Seedwing Policy Server. It is a WebAssembly (WASM) based
"single page application" written in Rust.

NOTE: The server binary also contains some server rendered pages, these are different and not part of this crate.

## Build overview

The crate `seedwing-policy-frontend` contains the web application. Using [trunk](https://trunkrs.dev) it can be compiled
into a ready to run WASM based distribution. This distribution is also embedded into the static server binary.

In order to embed this, the project `seedwing-policy-server/embedded-frontend` will build the frontend and
embed the output `dist/` folder using the `static_files` crate. The `embedded-frontend` crate is a library, which
only wraps the content. It is used by the `seedwing-policy-server` application to serve those files.

With this approach it is possible to run the console "stand-alone", using `trunk serve`. But at the same
time embed the distribution into the static binary. Externalizing the files into the `embedded-frontend` crate
allows the build to isolate running `trunk build`, caching the output, and speeding up the build when making
changes to the actual server.

## Development workflow

Depending on the component you want to work on, the workflow may be different.

## Working on the frontend

In order to work on the frontend you need to:

* Start the policy server (from the repository root):

  ```shell
  cargo run -p swio serve
  ```

* Materialize the NodeJS dependencies (at least once)

  ```shell
  cd frontend
  npm ci
  ```

* Run the frontend in development mode

  ```shell
  cd frontend
  trunk serve
  ```

* Navigate to the URL exposed by `trunk`. Normally this is http://localhost:8010.

Trunk will proxy all API requests to the server on `localhost:8080`, which is the one run by `cargo run`. It will
also pick up changes in the code base of the frontend crate, recompile, and reload.

## Working on the backend

If you are only working on the backend/server, just run the server as usual using `cargo run`. This will embed
the console automatically. However, it will not watch and refresh the frontend codebase.

## Running the console in production

The console is intended to be run through the server binary, which embeds a release build of the console.
