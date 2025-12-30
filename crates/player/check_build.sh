#!/usr/bin/env bash
cd /home/otto/repos/WrldBldr/Player
cargo check 2>&1 | tail -100
