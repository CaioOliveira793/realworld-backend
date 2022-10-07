#!/usr/bin/env sh

# `source ./script.sh` to make the functions available in your shell

set -o errexit

open_api_docs() {
	swagger-ui https://raw.githubusercontent.com/gothinkster/realworld/main/api/openapi.yml --allow-scripts --do-not-open
}
