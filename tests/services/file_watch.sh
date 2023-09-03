#!/usr/bin/env bash

set -e
set -u

output_file="$1"
shift
program=("$@")

running=true
trap 'running=false' EXIT INT TERM

while "$running"; do
  "${program[@]}" > "$output_file"
  sleep 1 || :
done

rm -f "$output_file"
