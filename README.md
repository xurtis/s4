The seL4 project management tool
================================

The `s4` command-line tool simplifies working with the seL4 projects
that live in the `SEL4` and `SEL4PROJ` organisation.

To use `s4`, you must have [`docker`][docker] installed or
[`podman`][podman] masquerading as `docker`. This is used to construct a
build environment consistent with those used to develop the projects.

The purpose of this tool is to wrap other tools such as [`repo`][repo],
[`cmake`][cmake], and the build toolchains to make working in various
projects easier.

[docker]: https://www.docker.com/
[podman]: https://podman.io/
[repo]: https://gerrit.googlesource.com/git-repo/+/refs/heads/master/README.md
[cmake]: https://cmake.org/
