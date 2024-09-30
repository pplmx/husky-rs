# Development Guide for husky-rs

Welcome to the development guide for `husky-rs`!
This document will walk you through setting up your development environment, running tests, building the project, and maintaining code quality.

## Table of Contents

- [Setting Up the Development Environment](#setting-up-the-development-environment)
    - [Prerequisites](#prerequisites)
    - [Installation Steps](#installation-steps)
- [Running Tests](#running-tests)
- [Building the Project](#building-the-project)
- [Code Style and Linting](#code-style-and-linting)
- [Generating Documentation](#generating-documentation)

## Setting Up the Development Environment

1. **Clone the Repository**: Start by cloning the project repository to your local machine and navigate to the project directory:

    ```bash
    git clone https://github.com/pplmx/husky-rs.git
    cd husky-rs
    ```

2. **Build the Project**: This command compiles the project and its dependencies:

    ```bash
    make build
    ```

## Running Tests

Tests are crucial to ensure the stability of the project. To run all tests, use the following command:

```bash
make test
```

This command will compile the code and run all tests, ensuring all components work as expected.

[Consider adding specific details on the structure of tests, testing strategy, or how to add new tests.]

## Building the Project

To build the project in release mode, use:

```bash
make build
```

This command will generate an optimized executable in the `target/release` directory.

## Code Style and Linting

Maintaining consistent code style is essential. We use `rustfmt` for formatting and `clippy` for linting.

To format your code, run:

```bash
make fmt
```

To run the linter:

```bash
make clippy
```

These commands will automatically check and optionally fix any code style issues according to the project's style guide.

## Generating Documentation

To generate the project documentation, run:

```bash
make doc
```

This will create the documentation for your project and its dependencies.

## Cleaning Up

To clean up build artifacts and other generated files, you can use:

```bash
make clean
```

This will remove the `target` directory and clean up any Docker-related resources.
