#!/usr/bin/env bash
mkdir -p ~/.config/user-tmpfiles.d && ln -sf "$(dirname "$(realpath "$0")")/grammar.conf" ~/.config/user-tmpfiles.d/
