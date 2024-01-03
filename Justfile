# Copyright 2023 Sean Kelleher. All rights reserved.
# Use of this source code is governed by an MIT
# licence that can be found in the LICENCE file.

# Note that `target` is used as the output directory for Rust so care should be
# taken that collisions don't occur between Rust output and local output.
tgt_dir := join(justfile_directory(), 'target')

# List available recipes.
default:
    just --list

# Run all checks.
check *tests: && check_style check_lint
    just check_unit {{tests}}

# Check for style issues.
check_style:
    make '{{tgt_dir}}/deps/dpnd/scripts/check_line_length.py'
    python3 '{{tgt_dir}}/deps/dpnd/scripts/check_line_length.py' \
        'src/**/*.rs' \
        79
    comment_style comment_style.yaml

# Check for semantic issues.
check_lint:
    @# We allow `clippy::just-underscores-and-digits` because the code generated
    @# by LALRPOP creates variables that are denied by this lint.
    @#
    @# We allow `manual-assert` because it allows us to use `panic!`s in
    @# `if`-statements, which generally results in shorter lines and fewer lines
    @# than we get when using `assert!`.
    @#
    @# TODO Consider denying `clippy::module-name-repetitions`.
    cargo clippy \
        --all-targets \
        --all-features \
        -- \
        --deny warnings \
        --deny clippy::pedantic \
        --deny clippy::cargo \
        --allow clippy::manual-assert \
        --allow clippy::module-name-repetitions

# Run unit tests.
check_unit tests='':
    cargo test {{tests}}

# Install project dependencies.
install_deps:
    dpnd install
