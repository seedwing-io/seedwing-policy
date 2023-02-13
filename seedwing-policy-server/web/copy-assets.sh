#!/usr/bin/env bash

set -e

TARGET="dist"

rm -Rf $TARGET

#
# We don't have any bundling tool like trunk, or in the JS world, webpack. So `cp` it is.
#

# asciidoctor

mkdir -p $TARGET/asciidoctor
cp -av node_modules/@asciidoctor/core/dist/browser/* $TARGET/asciidoctor/

# jquery

mkdir -p $TARGET/jquery
cp -av node_modules/jquery/dist/* $TARGET/jquery/

# patternfly

mkdir -p $TARGET/patternfly
cp -av node_modules/@patternfly/patternfly/patternfly*.css $TARGET/patternfly/

# monaco

mkdir -p $TARGET/monaco-editor/min/
cp -av node_modules/monaco-editor/min/* $TARGET/monaco-editor/min/


mkdir -p $TARGET/patternfly/assets/fonts
# rsync might be more powerful, but is also more complex, and even less available. So let's try to get the job done
# with find.
pushd node_modules/@patternfly/patternfly/ || exit 1
find assets \( -name "*.woff" -or -name "*.woff2" \) -exec install -pvD "{}" "../../../$TARGET/patternfly/{}"  \;
popd || exit 1
