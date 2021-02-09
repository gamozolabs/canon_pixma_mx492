# Entropy

The entropy of the firmware seems to follow

```
00000000 00020000 - Low entropy - Seems to be used for persistant storage, I
                                  see some SSIDs and WIFI passwords in here.
                                  Lots of this flash is unused.
00020000 00025000 - Med entropy - Initial boostrap/entry point code
00025000 00260000 - Hi  entropy - zlib compressed kernel payload (decomp at
                                  `0x003a2e82`, use `decompress_zlib.py`)
00260000 00280000 - Hi  entropy - Unknown payload (decomp at `0xf002024c`
                                  use `decompress_rle.c`)
00280000 00281000 - Low entropy - Code, decompression routines for `0x281000`
00281000 00b10976 - Hi entropy  - Compressed payload, (decomp at `0xf028088e`
                                  use `decompress_rle.c`)
00b10976 00c81000 - Empty       - Filled with FFs (unused flash)
00c81000 00ca2198 - Hi entropy  - Unknown, maybe code?
00ca2198 f0ff0000 - Empty       - Filled with FFs (unused flash)
f0ff0000 f1000000 - Low entropy - Reset vectors and boot code

```

# Okay

Vectors are at `0xf0ff0000`, which is likely actually `0xffff0000`

The flash seems to be mapped to `0xf0000000` and the entry point seems to be
`0xf0020000`. It seems the flash may also be aliased at `0xf1000000`

The bootstrap code quickly copies flash contents directly from `0xf00207f0` to
RAM at address `0x03a0000` for 14964 bytes. This contains decompression code
for the next stage, and is simply a zlib decompression.

Once the copy is complete, we branch into this new copied memory in RAM to
address `0x3a302e`

This decompresses multiple zlib payloads which start at offset `0x25000` in the
flash. This has a "special" format which is quite simple. It is simply

`[size_of_compressed_payload: u32][compressed_payload]`

This sequence repeats multiple times, in our case 4 times. The decompressed
size of the entire blob for us was `0x36c318` bytes

This data is decompressed to `0x18000000` which seems to be another alias for
RAM, as we also can access this same memory at `0x00000000`

At this point we return back to flash and branch indirectly to `0x1363ac`, this
is the "entry point" to the payload we just decompressed with zlib. This seems
to be a majority of the kernel code and is accurately reflected with the high
entropy region shown with `binwalk -E` on the firmware between ~120k and ~2.4M.

Then, we seem to decompress via some form of RLE(?) the data present at
`0x2602d4`. This can be decompressed with the code in `decompress_rle.c` and
the decompressed payload is placed at `0x184d5000`. In our case, the
decompressed size was 202,688 bytes and the compressed size was 86,513 bytes.
The decompression code can be found at `0xf002024c`

# New notes from reset vector

Start execution at `0xf0ff0000`

In function at `0xf0ff1854`
	Copy 0x97 bytes from `0xf0ff1a28` to `0x20000000`
	Copy 0xb294 bytes from `0xf0ff1ac0` to `0x20001000`

A checksum is done on `0xf0ff0000` to `0xf1000000` where all bytes are summed
with a wrapping add. At the end, it is expected that the 8-bit checksum results
in a zero, if it is not zero, the processor infinitely loops. This checksum
is implemented in `0xf02809f8`

