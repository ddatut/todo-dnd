#!/bin/bash
mkdir -p src
cat parts/main_part_00 parts/main_part_01 parts/main_part_02 parts/main_part_03 parts/main_part_04 parts/main_part_05 > src/main.rs
echo "Assembled src/main.rs"
