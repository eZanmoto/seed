Seed
====

About
-----

This project defines the Seed programming language, which is a stripped-down,
dynamically typed language with a C-style syntax. Seed is primarily intended to
be used as a base for prototyping programming languages and experimenting with
different language ideas, and provides a number of relatively standard
constructs found in many mainstream C-style languages. It is not intended for
production use.

```seed
print("Hello, world!");
```

Overview
--------

### Goals, non-goals and trade-offs

As with all projects, ideally Seed could do everything efficiently, cleanly and
safely. But as with all projects, this isn't possible in all cases, so when
different priorities are in contention, the design of Seed makes the following
trade-offs:

* Maintainability over performance
* General cases over edge-cases
* High-level composition and delegation over low-level processing

Installation
------------

At present, this project can only be used by building it from scratch. See the
"Build environment" and "Building" sections under "Development" for more
details.

### With Docker and [Dock](https://github.com/eZanmoto/dock)

If Docker and Dock are installed, then the following can be used to build the
project without needing to install any other tools:

```bash
dock run-in build-env: cargo build --locked
```

### Without Docker

The instructions in `build.Dockerfile` can be followed to prepare your local
environment for building the project. With the local environment set up, the
project can be built using `cargo build --locked`.

Usage
-----

When `seed` is built, it can be used to run a `.sd` script by passing it as the
first argument:

    seed hello.sd

Development
-----------

### Build environment

The build environment for the project is defined in `build.Dockerfile`. The
build environment can be replicated locally by following the setup defined in
the Dockerfile, or Docker can be used to mount the local directory in the build
environment by running `dock`.

### Building

The project can be built locally using `cargo build --locked`, or can be built
using `dock` by running the following:

    dock run-in build-env: cargo build --locked

### Testing

The project can be tested locally using `just check`, or the tests can be run
using `dock` by running the following:

    dock run-in build-env: just check

A subset of integration tests can be run by passing name patterns to `just`:

    just check add

The commands above will run all integration tests whose name contains "add".
