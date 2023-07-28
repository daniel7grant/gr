# Changelog

## [Unreleased]

-   Add GitHub Enterprise integration
-   Add assumptions to VCS type if the hostname matches "github", "gitlab" or "gitea"

## [0.2.0] - 2023-05-29

-   Add Gitea integration
-   Add repo subcommand to handle repositories
    - Create new repositories on the remote with repo create subcommand
    - Fork repositories with repo fork subcommand
    - Clone repositories to specific directories
    -   Add hidden repository delete command to repo
-   Allow type to be defined at login

## [0.1.5] - 2023-03-22

-   Add integration (end-to-end) tests for GitHub, GitLab and Bitbucket integrations
-   Add test running to GitHub Workflows
-   Refactor ureq calls to handle 400-500 status code errors gracefully
-   Fix Bitbucket pull request list parameters to avoid an infinite loop
-   Fix Github pull request querying by adding the repo orgname to the branch name
-   Fix README sync with npm package in pipeline
-   Add metadata for binstall to the Cargo.toml file

## [0.1.4] - 2023-03-17

### Fixed

-   Fix binary path on NPM packaged binaries again
-   Improve error handling for install package

## [0.1.3] - 2023-03-17

### Fixed

-   Fix binary path on NPM packaged binaries
-   Remove installation error in JS script with old Node version

### Improved

-   Rethink packages to improve compile speed
    -   Improve build times by ~70%
    -   Reduce binary size by ~75%
    -   Move from async to sync to avoid runtime (reqwest to ureq)
    -   Remove vendored libraries, rely on system libraries (e.g. native ssl)
    -   Remove git2, and move to git command parsing
    -   Remove color-eyre, add manual error printing

## [0.1.2] - 2023-03-11

### Fixed

-   Fix executable permission on NPM packaged binaries
-   Add CHANGELOG

## [0.1.1] - 2023-03-11

### Added

-   Add NPM installation option
-   Add README.md

## [0.1.0] - 2023-03-11

Initial release of the `gr` binary.

### Added

-   Integration with Github, GitLab (cloud or self-hosted) or Bitbucket
-   Create pull requests
    -   Allow creation with only a title
    -   Autogenerate pull requests descriptions if empty
    -   Read pull request descriptions from the stdin
    -   Allow adding reviewers
-   Read, list and open existing pull requests in the browser
-   Approve, merge and decline pull requests
-   Git integration (read remotes, pull, branch change)
-   Tracing and JSON output
-   Installation with cargo or cargo-binstall
