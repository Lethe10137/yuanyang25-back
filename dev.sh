#!/bin/bash
diesel migration run
RUST_LOG=INFO MODE=dev cargo run