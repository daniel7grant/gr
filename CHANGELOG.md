# Changelog

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
