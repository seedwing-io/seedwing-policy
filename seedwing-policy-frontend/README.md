# Seedwing Policy Server Frontend

This is an addon frontend to the existing Seedwing Policy Server.

## Development workflow

You will `trunk`, as described in the [main README](../README.md#development-setup).

In order to work on the frontend you need to:

* Start the policy server

  ```shell
  cargo run --package seedwing-policy-server
  ```

* Materialize the NodeJS dependencies (at least once)

  ```shell
  cd seedwing-policy-frontend
  yarn install
  ```

* Run the frontend in development mode

  ```shell
  cd seedwing-policy-frontend
  trunk serve
  ```