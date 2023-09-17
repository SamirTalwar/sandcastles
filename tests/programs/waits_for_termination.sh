#!/usr/bin/env bash

trap 'kill $program; exit 0' TERM
sleep 60 & program=$!
wait $program
