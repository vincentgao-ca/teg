#!/bin/bash
set -e

teg_VERSION=`node -e "console.log(require('./packages/teg-core/package.json').version);"`;
# echo "\e[32mSnapping Teg $teg_VERSION for x64:\e[0m Ignore the warnings and scary yellow text. This is not a pretty process.\n"
echo "Snapping Teg $teg_VERSION for x64: Ignore the warnings and scary yellow text. This is not a pretty process."
#
# yarn pkg:build
# yarn tegmarlin:build

cd ./snap

snapcraft clean
snapcraft clean teg

# sed -i -E "s/^version:[^\n]+/version: $teg_VERSION/g" ./snapcraft.yaml
# sed -i -E "s/run-on:[^\n]+/run-on: amd64/g" ./snapcraft.yaml

# rm -rf ./node_modules
#
# rsync -a --include="*.node" --include="*/" --exclude="*" --prune-empty-dirs ../node_modules ./

snapcraft --debug

mv ./*.snap ../dist/

echo "\n\nBUILD COMPLETE (don't worry about the warnings and yellow text they are probably fine)"
