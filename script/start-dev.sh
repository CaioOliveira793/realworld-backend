#!/usr/bin/env bash

set -o errexit

# Load environment variables from .env file
export $(grep -v '^#' .env | xargs);

cargo run;
