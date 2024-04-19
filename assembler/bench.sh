#!/bin/bash

echo -n "" > results.txt

for (( i = 0; i <= $1; i++ ))
do
    cargo run -- perf_test.gf 2>/dev/null | awk '{print $1}' >> results.txt
done
