#!/bin/bash -e

wasm-pack build --target web
sed 's/.\/pkg/./g' index.html > pkg/index.html
miniserve pkg --index index.html
