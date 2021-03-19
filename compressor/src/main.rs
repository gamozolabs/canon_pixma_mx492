use std::collections::BTreeSet;

/// Compute the theoretical compression ratio for a blob which is encoded with
/// specific parameters.
fn compute_compression_ratio(
        encodable_raw: usize, matching: usize, offset: usize) -> f64 {
    // Determine the prefix reference sizing to use
    // Note: There is an implicit +2 to reference lengths
    let (_, extra_ref_size) = if matching < 3 {
        // Cannot reference a sequence less than 3 bytes, thus, we must
        // encode an empty length
        (0, Some(0))
    } else if matching <= (15 + 2) {
        // We can encode it without an extra byte
        ((matching - 2) as u8, None)
    } else {
        (0, Some((matching - 2) as u8))
    };

    // Determine the prefix sizing to use
    let (_, extra_size) = if encodable_raw <= 2 {
        // We can safely encode a 1 or 2 byte length with the prefix
        ((encodable_raw + 1) as u8, None)
    } else {
        // Cannot encode the size we want in the prefix, so use an extra
        // 8-bit value
        (0, Some((encodable_raw + 1) as u8))
    };

    // Determine the encoding for the window offset
    let (_, byte_off, large_off) = if extra_ref_size == Some(0) {
        // No referenced data will be used
        (0, None, None)
    } else if offset <= (256 * 2 + 255) {
        // We can use the prefix to encode the offset
        ((offset / 256) as u8, Some(offset % 256), None)
    } else {
        // We must use an extra byte for the offset
        (3, Some(offset % 256), Some(offset / 256))
    };

    let mut uncompressed = encodable_raw; // Bytes encoded raw
    if extra_ref_size != Some(0) {
        uncompressed += matching;
    }

    let compressed = 1 + encodable_raw +
        extra_ref_size.is_some() as usize +
        extra_size.is_some() as usize +
        byte_off.is_some() as usize +
        large_off.is_some() as usize;

    compressed as f64 / uncompressed as f64
}

/// Compression implementation of the decompressor found at `0xf028030c`
fn compress(mut data: &[u8]) -> Vec<u8> {
    let mut ret = Vec::new();

    let mut in_window = (0..16 * 1024 * 1024).map(|_| {
        BTreeSet::new()
    }).collect::<Vec<_>>();

    let mut last_window_start = 0;
    let mut last_window_end = 0;
    let orig = data;
    while data.len() > 0 {
        let win_end   = orig.len() - data.len();
        let win_start = win_end.saturating_sub(256 * 255 + 255);

        for ii in last_window_end..win_end {
            let key = u32::from_le_bytes([
                orig.get(ii + 0).copied().unwrap_or(0),
                orig.get(ii + 1).copied().unwrap_or(0),
                orig.get(ii + 2).copied().unwrap_or(0),
                0,
            ]);
            in_window[key as usize].insert(ii);
        }

        for ii in last_window_start..win_start {
            let key = u32::from_le_bytes([
                orig.get(ii + 0).copied().unwrap_or(0),
                orig.get(ii + 1).copied().unwrap_or(0),
                orig.get(ii + 2).copied().unwrap_or(0),
                0,
            ]);
            in_window[key as usize].remove(&ii);
        }
       
        last_window_start = win_start;
        last_window_end = win_end;

        // Tuple containing
        // (largest number of matching bytes, window index, raw_size)
        let mut biggest_match = (0, 0, 0, std::f64::MAX);
        for raw_size in 0..=data.len().min(16) {
            // The maximum raw size we can encode is `std::u8::MAX - 1`
            // This is the number of bytes we will "consume" from data and place
            // into the output
            let encodable_raw = raw_size.min(std::u8::MAX as usize - 1);

            // Determine the window of data which can be referenced
            let win_end   = orig.len() - data.len() + encodable_raw;
            let win_start = win_end.saturating_sub(256 * 255 + 255);
            let window    = &orig[win_start..win_end];

            // Get the next data slice
            let next_data = &data[encodable_raw..];
           
            // Compute the reference-less compression ratio, as it might be
            // the best compression
            let compr = compute_compression_ratio(encodable_raw, 0, 0);
            if compr < biggest_match.3 {
                biggest_match = (0, 0, encodable_raw, compr);
            }

            // Find the largest match in the window
            let key = u32::from_le_bytes([
                next_data.get(0).copied().unwrap_or(0),
                next_data.get(1).copied().unwrap_or(0),
                next_data.get(2).copied().unwrap_or(0),
                0,
            ]);

            for &ii in in_window[key as usize].iter().rev().take(64) {
                // This can happen based on if we want to reference data
                // which is early in the window, but the window has
                // slightly shifted due to raw data being encoded
                if ii < win_start {
                    continue;
                }

                assert!(ii < win_end);

                let ii = ii - win_start;

                // Get the number of matching bytes at this index in the window
                let matching = window[ii..].iter().zip(next_data)
                    .take_while(|(a, b)| a == b).count()
                    .min(0xff + 2); // Cannot encode larger than this anyways

                let offset = window.len() - ii;
                
                let compr = compute_compression_ratio(
                    encodable_raw, matching, offset);
                if compr < biggest_match.3 {
                    biggest_match =
                        (matching, offset, encodable_raw, compr);
                }
            }
            
            for ii in last_window_end..win_end {
                let ii = ii - win_start;

                // Get the number of matching bytes at this index in the window
                let matching = window[ii..].iter().zip(next_data)
                    .take_while(|(a, b)| a == b).count()
                    .min(0xff + 2); // Cannot encode larger than this anyways

                let offset = window.len() - ii;
                
                let compr = compute_compression_ratio(
                    encodable_raw, matching, offset);
                if compr < biggest_match.3 {
                    biggest_match =
                        (matching, offset, encodable_raw, compr);
                }
            }
        };

        // The maximum raw size we can encode is `std::u8::MAX - 1`
        // This is the number of bytes we will "consume" from data and place
        // into the output
        let raw_size = biggest_match.2;
        let encodable_raw = raw_size.min(std::u8::MAX as usize - 1);

        // Determine the prefix reference sizing to use
        // Note: There is an implicit +2 to reference lengths
        let (prefix_ref_size, extra_ref_size) = if biggest_match.0 < 3 {
            // Cannot reference a sequence less than 3 bytes, thus, we must
            // encode an empty length
            (0, Some(0))
        } else if biggest_match.0 <= (15 + 2) {
            // We can encode it without an extra byte
            ((biggest_match.0 - 2) as u8, None)
        } else {
            (0, Some((biggest_match.0 - 2) as u8))
        };

        // Determine the prefix sizing to use
        let (prefix_size, extra_size) = if encodable_raw <= 2 {
            // We can safely encode a 1 or 2 byte length with the prefix
            ((encodable_raw + 1) as u8, None)
        } else {
            // Cannot encode the size we want in the prefix, so use an extra
            // 8-bit value
            (0, Some((encodable_raw + 1) as u8))
        };

        // Determine the encoding for the window offset
        let (prefix_off, byte_off, large_off) = if extra_ref_size == Some(0) {
            // No referenced data will be used
            (0, None, None)
        } else if biggest_match.1 <= (256 * 2 + 255) {
            // We can use the prefix to encode the offset
            ((biggest_match.1 / 256) as u8, Some(biggest_match.1 % 256), None)
        } else {
            // We must use an extra byte for the offset
            (3, Some(biggest_match.1 % 256), Some(biggest_match.1 / 256))
        };

        // Construct the prefix and push it
        let prefix =
            (prefix_ref_size << 4) | (prefix_off << 2) | (prefix_size << 0);
        ret.push(prefix);

        // Encode the extra size if we need it
        if let Some(val) = extra_size {
            ret.push(val);
        }

        // Encode the extra reference size if we need it
        if let Some(val) = extra_ref_size {
            ret.push(val);
        }

        // Encode the data
        ret.extend_from_slice(&data[..encodable_raw]);
        data = &data[encodable_raw..];

        // Encode the offset byte
        if let Some(val) = byte_off {
            ret.push(val as u8);
        }

        // Encode the extra offset byte
        if let Some(val) = large_off {
            ret.push(val as u8);
        }

        if extra_ref_size != Some(0) {
            // Advance the data by the amount referenced
            data = &data[biggest_match.0..];
        }
    }

    ret
}

/// Implementation of the decompression code which is found at `0xf028030c`
fn decompress(mut data: &[u8]) -> Vec<u8> {
    let mut ret = Vec::new();

    loop {
        // Read the prefix byte
        // [ref_length:4][ref_off_256:2][length:2]
        let prefix = data[0];
        data = &data[1..];

        // Read the length
        let mut length = prefix & 3;
        if length == 0 {
            length = data[0];
            data = &data[1..];
        }

        let mut ref_length = prefix >> 4;
        if ref_length == 0 {
            ref_length = data[0];
            data = &data[1..];
        }

        // Copy the bytes
        ret.extend_from_slice(&data[..length as usize - 1]);
        data = &data[length as usize - 1..];

        if ref_length != 0 {
            // Get the byte offset
            let offset = data[0] as usize;
            data = &data[1..];

            let mut offset_256 = ((prefix >> 2) & 3) as usize;
            if offset_256 == 3 {
                offset_256 = data[0] as usize;
                data = &data[1..];
            }

            let reference = ret.len() - (offset_256 * 256 + offset);
            for ii in 0..ref_length as usize + 2 {
                ret.push(ret[reference + ii]);
            }
        }

        if ret.len() == 0xfe5b20 {
            break;
        }
    }

    ret
}

fn main() -> std::io::Result<()> {
    // Read the firmware
    let mut firmware =
        std::fs::read("../firmware_dumps/canon_pixma_mx492.flashbin.BIN")?;
    
    // Get a slice to the region which can contain the compressed data
    let compress_region = &mut firmware[0x281000..0xc81000];

    // Decompress the original blob
    let mut decomp = decompress(compress_region);
    
    // Patch location
    // Change mov r2, #0x10 -> mov r2, #0xff
    // This is the size of the shellcode that we execute including null
    // terminator. Thus, we get 254 bytes of arbitrary data to execute
    decomp[0x000e0df4] = 0xff;

    // Change blx r3 -> blx r2
    // This will cause us to jump into our shellcode
    decomp[0x00e0e1c] = 0x90;

    // Re-compress the blob with our algo
    let it = std::time::Instant::now();
    let comp = compress(&decomp);
    print!("Compression took {:?}\n", it.elapsed());
    print!("Compression ratio {}\n", comp.len() as f64 / decomp.len() as f64);

    // Make sure we round tripped
    assert!(decompress(&comp) == decomp);

    // Replace the old contents with our new compression data
    compress_region[..comp.len()].copy_from_slice(&comp);

    // Write out our version of the firmware
    std::fs::write("our_firmware.bin", &firmware)?;

    Ok(())
}
