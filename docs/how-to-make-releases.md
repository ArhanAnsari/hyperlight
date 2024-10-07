# Releasing a new Hyperlight version to Cargo

This document details the process of releasing a new version of Hyperlight to the [Azure-internal Cargo feeds](https://dev.azure.com/AzureContainerUpstream/hyperlight/_artifacts/feed/hyperlight_packages). It's intended to be used as a checklist for the developer doing the release. The checklist is represented in the below sections.

## Update Cargo.toml Versions

Currently, we need to manually update the `Cargo.toml` version numbers to match to whatever release we are making. This can be done with the following command (replacing `0.4.0` with your new version):
```bash
cargo ws version --force=hyperlight_* --no-git-commit --yes custom 0.4.0
```

> Note 1: we'll use `v0.4.0` as the version for the above and all subsequent instructions. You should replace this with the version you're releasing. Make sure your version follows [SemVer](https://semver.org) conventions as closely as possible, and is prefixed with a `v` character. *In particular do not use a patch version unless you are patching an issue in a release branch, releases from dev should always be minor or major versions* (see [here](https://github.com/deislabs/hyperlight/issues/1072) for reasons why).

> Note 2: If you don't have cargo-workspaces installed, you can do so with: `cargo install cargo-workspaces`

Create a PR with these changes and merge them into the dev branch.

## Create a tag

When the above PR has merged into `dev` branch you should create a tag. ***Make sure you have pulled the recently updated `dev` branch***, and do the following on the `dev` branch:

```bash
git tag -a v0.4.0 -m "A brief description of the release"
git push origin v0.4.0 # if you've named your git remote for the deislabs/hyperlight repo differently, change 'origin' to your remote name
```

If you are creating a patch release see the instructions [here](#patching-a-release).

## Create a release branch (no manual steps)

After you push your new tag in the previous section, the ["Create a Release Branch"](https://github.com/deislabs/hyperlight/actions/workflows/CreateReleaseBranch.yml) CI job will automatically run. When this job completes, a new `release/v0.4.0` branch will be automatically created for you.

## Create a new GitHub release

After the previous CI job runs to create the new release branch, go to the ["Create a Release"](https://github.com/deislabs/hyperlight/actions/workflows/CreateRelease.yml). GitHub actions workflow and do the following:

1. Click the "Run workflow" button near the top right
2. In the Use workflow from dropdown, select the `release/v0.4.0` branch
3. Click the green **Run workflow** button

> Note: In case you see a "Create a Release" job already running before starting this step, that is because the "Create a Release" workflow also automatically runs on push to `dev` branch to create a pre-release. You must still do the steps outlined above.

When this job is done, a new [GitHub release](https://github.com/deislabs/hyperlight/releases) will be created for you. This job also publishes the following rust packages to the internal cargo feed:
- `hyperlight_capi`
- `hyperlight_common`
- `hyperlight_guest`
- `hyperlight_guestlib_c_includes`
- `hyperlight_host`

 The GitHub release also houses other artifacts (e.g. `simpleguest.exe`, `callbackguest.exe`, etc.) and also to creates nuget packages, which, are currently used to distribute native libraries and headers to projects that consume Hyperlight, this will change in the future when [this issue](https://github.com/deislabs/hyperlight/issues/512) has been addressed.

## Patching a release

If you need to update a previously released version of Hyperlight then you should open a Pull Request against the release branch you want to patch, for example if you wish to patch the release `v0.4.0` then you should open a PR against the `release/v0.4.0` branch.

Once the PR is merged, then you should follow the instructions above. In this instance the version number of the tag should be a patch version, for example if you are patching the `release/v0.4.0` branch and this is the first patch release to that branch then the tag should be `v0.4.1`. If you are patching a patch release then the tag should be `v0.4.2` and the target branch should be `release/v0.4.1` and so on.
