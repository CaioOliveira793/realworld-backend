#!/usr/bin/env bash

set -o errexit

swagger-cli validate docs/openapi.main.yml

swagger-cli bundle docs/openapi.main.yml --type yaml --outfile out/openapi.yml
