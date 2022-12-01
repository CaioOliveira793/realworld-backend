#!/usr/bin/env bash

set -o errexit

cargo fmt --check;

cargo clippy "$@";
