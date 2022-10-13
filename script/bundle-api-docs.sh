#!/usr/bin/env bash

set -o errexit

swagger-cli validate docs/open_api.yml

swagger-cli bundle docs/open_api.yml --type yaml --outfile out/openapi.yml
