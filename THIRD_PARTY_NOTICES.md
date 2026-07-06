# Third-Party Notices

DustBunny's scanner backend is adapted from the design of `du-dust`, the maintained Rust `dust` project:

- Project: https://github.com/bootandy/dust
- Crate package: `du-dust`
- License: Apache-2.0

The upstream `du-dust` crate is published as a binary-only package, so DustBunny keeps a local backend that follows the same core scanner approach while converting results into DustBunny's TUI model.
