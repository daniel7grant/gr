# gr

Interact with remote repositories like you interact with git

## Features

-   Login with Github, GitLab (cloud or self-hosted) or Bitbucket
-   Create new PRs with only a title
-   Read and list existing PRs
-   Approve, merge and decline PRs
-   All with git integration (pull, branch change)

All from the terminal!

## Installation

You can install with [cargo](https://rustup.rs/):

```shell
cargo install gr-bin
```

Or faster, with [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```shell
cargo binstall gr-bin
```

## Usage

`gr` is similar to `git`, that it looks at your current directory, and reads the information from git. To start, move to a local git repo, and login to the remote:

```shell
cd /path/to/repo
gr login
```

Create pull request on current branch:

```shell
gr pr create -m "PR title"
```

Get information about the open PRs:

```shell
gr pr list
```

Merge the PR on the current branch:

```shell
gr pr merge --delete
```

For more information, print the help with `gr --help`.