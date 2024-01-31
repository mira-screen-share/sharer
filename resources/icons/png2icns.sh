#!/bin/sh -x

set -e

SIZES="
16,16x16
32,16x16@2x
32,32x32
64,32x32@2x
128,128x128
256,128x128@2x
256,256x256
512,256x256@2x
512,512x512
1024,512x512@2x
"

for IN in "$@"; do
    BASE=$(basename "$IN" | sed 's/\.[^\.]*$//')
    ICONSET="$BASE.iconset"
    mkdir -p "$ICONSET"
    for PARAMS in $SIZES; do
        SIZE=$(echo $PARAMS | cut -d, -f1)
        LABEL=$(echo $PARAMS | cut -d, -f2)
        # resize png
        convert -background none -resize ${SIZE}x${SIZE} "$IN" "$ICONSET"/icon_${LABEL}.png
    done

    iconutil -c icns "$ICONSET"

    for PARAMS in $SIZES; do
        mv "$ICONSET"/icon_$(echo $PARAMS | cut -d, -f2).png $(echo $PARAMS | cut -d, -f2).png
    done

    rm -r "$ICONSET"
done
