#!/bin/bash
diesel migration redo --all
RUST_LOG=INFO MODE=dev cargo run