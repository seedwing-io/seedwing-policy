#!/usr/bin/env bash

cp -av node_modules/@asciidoctor/core/dist/browser dist/asciidoctor
cp -av node_modules/jquery/dist dist/jquery

mkdir -p dist/patternfly
cp -av node_modules/@patternfly/patternfly/patternfly*.css dist/patternfly/
find node_modules/@patternfly/patternfly/assets \( -name "*.woff" -or -name "*.woff2" \) -exec cp "{}" dist/patternfly/ \;
