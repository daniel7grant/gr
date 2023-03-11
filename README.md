# gr

Interact with remote repositories like you interact with git

## Features

-   Login with Github, GitLab (cloud or self-hosted) or Bitbucket
-   Create new pull request with only a title
-   Read, list and open existing pull requests in the browser
-   Approve, merge and decline pull requests
-   With git integration (pull, branch change)

And it's all from the terminal!

## Installation

You can install with [cargo](https://rustup.rs/), [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) or [npm](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm):

```shell
# Any one of these is good
cargo install gr-bin
cargo binstall gr-bin
npm install --global gr-bin
```

If all went well, you should have `gr` installed:
```shell
gr --version
```

## Usage

`gr` is similar to `git`, that it looks at your current directory, and reads the information from git. To start, move to a local git repo, and login to the remote (if you only want to try, replace `gr` with `npx gr-bin`):

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
