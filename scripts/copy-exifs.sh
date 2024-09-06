#!/bin/bash

#### INFO: Requires async-cmd: `cargo install async-cmd`
#### INFO: You can also use `-x` option of the transform binary

S="cmd_socket"
async --socket "$S" server --start

for file in "$@"; do
    fname="${file##*/}"
    fstem="${fname%.*}"
    echo "Processing $fname"
    async --socket "$S" cmd -- sh -c 'exiv2 -ea- '"${file@Q}"' | exiv2 -ia- '"$fstem.tif"'; exiv2 -eX- '"${file@Q}"' | exiv2 -iX- '"$fstem.tif"
done

async --socket "$S" wait
async --socket "$S" server --stop
