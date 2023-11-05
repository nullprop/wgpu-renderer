#!/bin/bash -e

wasm-pack build --debug --target web
sed 's/.\/pkg/./g' index.html > pkg/index.html
miniserve pkg --index index.html
