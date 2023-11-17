#!/bin/sh
# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Must be in dodge-the-creep's rust directory in order to pick up the .cargo/config
cd `dirname "$0"`

# We build the host gdextension first so that the godot editor doesn't complain.
cargo +nightly build --package dodge-the-creeps &&
cargo +nightly build --package dodge-the-creeps --target wasm32-unknown-emscripten -Zbuild-std $@ &&
if [ -n "$GODOT4_BIN" ] ; then
    case $* in
        *--release* )
            echo Godot export release
            $GODOT4_BIN --headless --path ../godot --export-release Web
            ;;
        * )
            echo Godot export debug
            $GODOT4_BIN --headless --path ../godot --export-debug Web
            ;;
    esac
else
    echo ; echo No variable GODOT4_BIN found, skipping export.
fi
