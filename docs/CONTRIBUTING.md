# Versioning

This project follows [semantic versioning](https://semver.org/).

Some events may trigger a major (breaking) version like option deprecation or mandatory option.

# Architecture

osc-cost's code is organized in order to have:
- a choosen input (Outscale API, json, ...)
- a core which compute costs
- a choosen output format (hour, month, json, ...)

# Debuging

You can get more details by setting environement variable `RUST_LOG=debug`. Check [env_logger documentation](https://docs.rs/env_logger/0.9.3/env_logger/) for more details.

# Sending a Merge Request

If you plan to make some change in source code, consider making a pull request in [openapi-generator project](https://github.com/OpenAPITools/openapi-generator/).

Otherwise:
- Your merge request must be rebased on the latest commit.
- Be sure that tests still pass by running `make test`.

# How to release

Gitub bot should have produced a new version and creating the new release tag should push release to crate.io.

If this is not the case:
1. Be sure have the latest version from repository.
2. `make test` and fix any issue.
3. Update Cargo.toml with new version following [semantic versioning](https://semver.org/) and commit
4. PR, review and merge
5. Create new release