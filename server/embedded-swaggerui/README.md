# embedded-swaggerui

This crate embeds swaggerui, to be served by the seedwing server.

## Updating

All code is committed to the repository, so that it can simply the checked out and built.

Updating the dependency requires the following steps:

* Run the update command

  ```shell
  make update
  ```

* Add files to git

  ```shell
  git add dist
  ```
* Commit & push

* (optionally) clean up local intermediate files

  ```shell
  make clean
  ```
