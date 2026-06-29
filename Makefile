SHELL := /bin/bash

PROJECT_NAME_FROM_CARGO := $(shell sed -n 's/^[[:space:]]*name[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' Cargo.toml | head -1)
PROJECT_VERSION_FROM_CARGO := $(shell sed -n 's/^[[:space:]]*version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' Cargo.toml | head -1)
PROJECT_NAME ?= $(or $(PROJECT_NAME_FROM_CARGO),$(notdir $(CURDIR)))
PROJECT_VERSION ?= $(or $(PROJECT_VERSION_FROM_CARGO),dev)

CARGO := cargo
BIN := tyla

# `make run` converts this file by default. Override with `make run FILE=...`.
FILE ?= examples/sample.tex
# Arguments passed to the binary. Defaults to a full-document conversion of FILE.
# Override for anything else, e.g. `make run ARGS="info"` or `make run ARGS="--detect examples/sample.typ"`.
ARGS ?= -f $(FILE)

$(info ------------------------------------------)
$(info Project: $(PROJECT_NAME) v$(PROJECT_VERSION))
$(info ------------------------------------------)

.PHONY: build b run r demo test t test-all check fmt lint harden bench doc install clean help h

build:
	@$(CARGO) build

b: build

release:
	@$(CARGO) build --release

# Run the CLI. By default converts FILE (a full LaTeX/Typst document) to stdout.
#   make run                          # convert examples/sample.tex -> Typst
#   make run FILE=examples/sample.typ # convert a Typst doc -> LaTeX
#   make run ARGS="info"              # any other subcommand/flags
run:
	@$(CARGO) run --quiet --bin $(BIN) -- $(ARGS)

r: run

# Show a round-trip: LaTeX -> Typst and Typst -> LaTeX on the bundled examples.
demo:
	@echo "=== examples/sample.tex  (LaTeX -> Typst) ==="
	@$(CARGO) run --quiet --bin $(BIN) -- -f examples/sample.tex
	@echo
	@echo "=== examples/sample.typ  (Typst -> LaTeX) ==="
	@$(CARGO) run --quiet --bin $(BIN) -- -f examples/sample.typ

test:
	@$(CARGO) test

t: test

# Run every test, including doctests, under all feature combinations.
test-all:
	@$(CARGO) test --all-targets --all-features
	@$(CARGO) test --doc --all-features
	@$(CARGO) test --no-default-features

check:
	@$(CARGO) check --all-targets --all-features

fmt:
	@$(CARGO) fmt --all

lint:
	@$(CARGO) clippy --all-targets --all-features -- -D warnings

# CI-style gate: formatting, lints, and the full test suite.
harden:
	@$(CARGO) fmt --all -- --check
	@$(CARGO) clippy --all-targets --all-features -- -D warnings
	@$(CARGO) test --all-targets --all-features

bench:
	@$(CARGO) bench

doc:
	@$(CARGO) doc --no-deps --open

install:
	@$(CARGO) install --path .

clean:
	@$(CARGO) clean

help:
	@echo
	@echo "Usage: make [target]"
	@echo
	@echo "Available targets:"
	@echo "  build      Build the debug binary"
	@echo "  release    Build the optimized release binary"
	@echo "  run        Run $(BIN) on FILE (default: $(FILE))"
	@echo "  demo       Convert both bundled examples (LaTeX<->Typst round-trip)"
	@echo "  test       Run the test suite"
	@echo "  test-all   Run all tests incl. doctests and feature combinations"
	@echo "  check      Type-check all targets and features"
	@echo "  fmt        Format the code"
	@echo "  lint       Run clippy with warnings denied"
	@echo "  harden     fmt --check + clippy + full test suite (CI gate)"
	@echo "  bench      Run benchmarks"
	@echo "  doc        Build and open API docs"
	@echo "  install    Install the $(BIN) binary with cargo"
	@echo "  clean      Remove build artifacts"
	@echo
	@echo "Examples:"
	@echo "  make run"
	@echo "  make run FILE=examples/sample.typ"
	@echo "  make run ARGS=\"info\""
	@echo "  make run ARGS=\"--detect examples/sample.tex\""
	@echo "  make demo"
	@echo

h: help
