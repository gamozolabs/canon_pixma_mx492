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
```

# Okay

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

