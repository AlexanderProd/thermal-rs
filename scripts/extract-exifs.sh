#!/bin/bash

#### INFO: Requires async-cmd: `cargo install async-cmd`
#### INFO: You can use the `stats` binary directly with the rjpegs

S="cmd_socket"
async --socket "$S" server --start

for file in "$@"; do
    fname="${file##*/}"
    fstem="${fname%.*}"
    echo "Processing $fname"
    async --socket "$S" cmd -- sh -c 'exiftool -b -j '"${file@Q}"' > '"${fstem@Q}"'.json'
done

async --socket "$S" wait
async --socket "$S" server --stop
