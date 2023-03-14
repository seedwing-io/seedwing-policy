# Release process

A Seedwing release consists of the `swio` binary along with SBOM, attestations and container images hosted on ghcr.io.

The release process is built around automation in GitHub Actions. The following GHA workflows are involved:

* `ci` - building and testing the source
* `release` - build release artifacts and metadata, also runs `container` and `playground` workflows. Publishes artifacts and containers.
* `container` - builds container images.
* `playground` - triggers an update of https://playground.seedwing.io
* `nightly` - cron-job that tags a nightly release, which works like any regular release but with a special version.

## Creating a release

Follow these steps to create a release:

* Create a git tag
* Update Cargo.toml with 'next release' versions

NOTE: The last step is important to ensure nightly versions appear as being newer than the latest release.

### Create a git tag 

```shell
git tag 0.2.0 -m 'Tag new release'
git push --follow-tags
```


### Update Cargo.toml version

There are only a few files that needs updating (NOTE: List may be stale, make sure you check if there are addition .toml files):

* swio/Cargo.toml
* frontend/Cargo.toml
* server/Cargo.toml

Once updated, commit and push:

```shell
git add swio/Cargo.toml frontend/Cargo.toml server/Cargo.toml
git commit -m 'chore: update current version'
git push
```

## Updating the playground

The playground is automatically updated for any given release, which also means nightly releases will also update the playground automatically.

## Note on permissions

The pipelines mostly rely on standard builder tokens, but there are a few exceptions:

* NIGHTLY_TOKEN - this token is used by the nightly workflow to push the nightly tags.
* PLAYGROUND_TOKEN - this token is used to trigger the workflows in the playground.seedwing.io repository.
