//! Original TIM archive and chunk readers. No DOS execution is required.
mod resources;
pub use resources::Resources;
mod screen;
pub use screen::Screen;

use crate::storage as fs;
use std::{collections::BTreeMap, path::Path};
type Result<T> = std::result::Result<T, String>;
fn word(data: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        data.get(offset..offset + 4)
            .ok_or("Truncated integer")?
            .try_into()
            .unwrap(),
    ))
}
pub(crate) fn chunks<'a>(data: &'a [u8], output: &mut Vec<(&'a [u8], &'a [u8])>) -> Result<()> {
    let mut offset = 0;
    while offset < data.len() {
        let tag = data.get(offset..offset + 4).ok_or("Truncated chunk tag")?;
        let size = word(data, offset + 4)?;
        offset += 8;
        let payload = data
            .get(offset..offset + (size & 0x7fffffff) as usize)
            .ok_or("Truncated chunk")?;
        if size & 0x80000000 != 0 {
            chunks(payload, output)?;
        } else {
            output.push((tag, payload));
        }
        offset += payload.len();
    }
    Ok(())
}
/// TIM's least-significant-bit-first LZW stream grows from 9 to 12 bits.
pub(crate) fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    let expected = word(data, 1)? as usize;
    if expected > 16 * 1024 * 1024 {
        return Err("Resource exceeds size limit".into());
    }
    let input = data.get(5..).ok_or("Missing compressed data")?;
    match data[0] {
        0 => return Ok(input.to_vec()),
        1 => {
            let mut out = Vec::new();
            let mut offset = 0;
            while out.len() < expected {
                let count = *input.get(offset).ok_or("Truncated RLE")?;
                offset += 1;
                if count & 128 != 0 {
                    let value = *input.get(offset).ok_or("Truncated RLE value")?;
                    offset += 1;
                    out.extend(std::iter::repeat_n(value, (count & 127) as usize));
                } else {
                    let end = offset + count as usize;
                    out.extend_from_slice(input.get(offset..end).ok_or("Truncated RLE run")?);
                    offset = end;
                }
            }
            if out.len() != expected {
                return Err("RLE size mismatch".into());
            }
            return Ok(out);
        }
        2 => {}
        3 => return lzhuf(input, expected),
        compression => return Err(format!("Unsupported compression {compression}")),
    }
    let mut dictionary: Vec<Vec<u8>> = (0..256).map(|v| vec![v as u8]).collect();
    dictionary.push(Vec::new());
    let mut width = 9;
    let mut bit = 0;
    let mut group_bits = 0;
    let mut previous: Vec<u8> = Vec::new();
    let mut output = Vec::with_capacity(expected);
    while output.len() < expected {
        if bit + width > input.len() * 8 {
            return Err("Truncated LZW stream".into());
        }
        let mut code = 0;
        for shift in 0..width {
            code |= (((input[(bit + shift) / 8] >> ((bit + shift) % 8)) & 1) as usize) << shift;
        }
        bit += width;
        group_bits = (group_bits + width) % (width * 8);
        if code == 256 {
            if group_bits != 0 {
                bit += width * 8 - group_bits;
            }
            dictionary.truncate(257);
            width = 9;
            group_bits = 0;
            previous.clear();
            continue;
        }
        let value = if code < dictionary.len() {
            dictionary[code].clone()
        } else if code == dictionary.len() && !previous.is_empty() {
            let mut value = previous.clone();
            value.push(previous[0]);
            value
        } else {
            return Err(format!("Invalid LZW code {code}"));
        };
        if value.is_empty() {
            return Err("Empty LZW code".into());
        }
        output.extend_from_slice(&value);
        if !previous.is_empty() && dictionary.len() < 4096 {
            previous.push(value[0]);
            dictionary.push(previous);
            group_bits = 0;
            if dictionary.len() == 1 << width && width < 12 {
                width += 1;
            }
        }
        previous = value;
    }
    if output.len() != expected {
        return Err("LZW size mismatch".into());
    }
    Ok(output)
}
pub fn screen(resources: &Resources, image_name: &str, palette_name: &str) -> Result<Screen> {
    let mut image_chunks = Vec::new();
    chunks(resources.get(image_name)?, &mut image_chunks)?;
    let mut palette_chunks = Vec::new();
    chunks(resources.get(palette_name)?, &mut palette_chunks)?;
    let palette = palette_chunks
        .iter()
        .find(|(tag, _)| *tag == b"VGA:")
        .ok_or("Missing VGA palette")?
        .1;
    if palette.len() != 768 {
        return Err("Invalid palette size".into());
    }
    let dimensions = image_chunks
        .iter()
        .find(|(tag, _)| *tag == b"DIM:")
        .ok_or("Missing dimensions")?
        .1;
    let width = u16::from_le_bytes(
        dimensions
            .get(..2)
            .ok_or("Missing width")?
            .try_into()
            .unwrap(),
    ) as u32;
    let height = u16::from_le_bytes(
        dimensions
            .get(2..4)
            .ok_or("Missing height")?
            .try_into()
            .unwrap(),
    ) as u32;
    let mut pixels = vec![0u8; (width * height) as usize];
    for (tag, payload) in image_chunks {
        if tag != b"BIN:" && tag != b"VGA:" {
            continue;
        }
        let plane = decompress(payload)?;
        if plane.len() * 2 != pixels.len() {
            return Err("Screen plane size mismatch".into());
        }
        for (pair, byte) in pixels.chunks_exact_mut(2).zip(plane) {
            let shift = if tag == b"VGA:" { 4 } else { 0 };
            pair[0] |= (byte >> 4) << shift;
            pair[1] |= (byte & 15) << shift;
        }
    }
    let rgba = pixels
        .into_iter()
        .flat_map(|index| {
            let p = &palette[index as usize * 3..index as usize * 3 + 3];
            [vga_channel(p[0]), vga_channel(p[1]), vga_channel(p[2]), 255]
        })
        .collect();
    Ok(Screen {
        width,
        height,
        rgba,
    })
}

/// Sprite frames stored as scanline commands, with transparent skips.
pub fn sprites(resources: &Resources, name: &str, palette_name: &str) -> Result<Vec<Screen>> {
    let mut parts = Vec::new();
    chunks(resources.get(name)?, &mut parts)?;
    let info = parts
        .iter()
        .find(|(tag, _)| *tag == b"INF:")
        .ok_or("Missing sprite dimensions")?
        .1;
    let read_short = |offset: usize| -> Result<u16> {
        Ok(u16::from_le_bytes(
            info.get(offset..offset + 2)
                .ok_or("Truncated sprite dimensions")?
                .try_into()
                .unwrap(),
        ))
    };
    let count = read_short(0)? as usize;
    let mut palette_parts = Vec::new();
    chunks(resources.get(palette_name)?, &mut palette_parts)?;
    let palette = palette_parts
        .iter()
        .find(|(tag, _)| *tag == b"VGA:")
        .ok_or("Missing sprite palette")?
        .1;
    if parts.iter().any(|(tag, _)| *tag == b"BIN:") {
        let sizes: Vec<(usize, usize)> = (0..count)
            .map(|frame| {
                Ok((
                    read_short(2 + frame * 2)? as usize,
                    read_short(2 + count * 2 + frame * 2)? as usize,
                ))
            })
            .collect::<Result<_>>()?;
        let total: usize = sizes.iter().map(|(w, h)| w * h).sum();
        let mut indices = vec![0u8; total];
        for (tag, payload) in &parts {
            if *tag != b"BIN:" && *tag != b"VGA:" {
                continue;
            }
            let plane = decompress(payload)?;
            if plane.len() * 2 != total {
                return Err("Sprite plane size mismatch".into());
            }
            let shift = if *tag == b"VGA:" { 4 } else { 0 };
            for (pair, byte) in indices.chunks_exact_mut(2).zip(plane) {
                pair[0] |= (byte >> 4) << shift;
                pair[1] |= (byte & 15) << shift;
            }
        }
        let mut offset = 0;
        return sizes
            .into_iter()
            .map(|(width, height)| {
                let rgba = indices[offset..offset + width * height]
                    .iter()
                    .flat_map(|&index| {
                        let p = &palette[index as usize * 3..index as usize * 3 + 3];
                        [
                            vga_channel(p[0]),
                            vga_channel(p[1]),
                            vga_channel(p[2]),
                            if index == 0 { 0 } else { 255 },
                        ]
                    })
                    .collect();
                offset += width * height;
                Ok(Screen {
                    width: width as u32,
                    height: height as u32,
                    rgba,
                })
            })
            .collect();
    }
    let scanlines = parts
        .iter()
        .find(|(tag, _)| *tag == b"SCN:")
        .ok_or_else(|| format!("{name}: unsupported sprite encoding"))?
        .1;
    let offsets = parts
        .iter()
        .find(|(tag, _)| *tag == b"OFF:")
        .ok_or("Missing sprite offsets")?
        .1;
    let mut frames = Vec::new();
    for frame in 0..count {
        let offset = word(offsets, frame * 4)? as usize;
        let mut stream = scanlines
            .get(offset..)
            .ok_or("Invalid sprite offset")?
            .iter()
            .copied();
        let width = read_short(2 + frame * 2)? as usize;
        let height = read_short(2 + count * 2 + frame * 2)? as usize;
        let mut indices = vec![None; width * height];
        let base = stream.next().ok_or("Truncated SCN base")?;
        let (mut x, mut y, mut last) = (0i32, 0usize, 255u8);
        while y < height {
            let byte = stream.next().ok_or("Truncated SCN command")?;
            let command = byte & 192;
            let count = (byte & 63) as usize;
            match command {
                0 => {
                    if last == 0 && count != 0 {
                        x -= (count * 64) as i32;
                    } else {
                        y += 1;
                        x -= count as i32;
                    }
                }
                64 => {
                    if count == 0 {
                        break;
                    }
                    x += count as i32;
                }
                128 | 192 => {
                    let mut packed = 0;
                    for pixel in 0..count {
                        let color = if command == 128 {
                            if pixel == 0 {
                                packed = stream.next().ok_or("Truncated SCN repeat")?;
                            }
                            packed
                        } else {
                            if pixel % 2 == 0 {
                                packed = stream.next().ok_or("Truncated SCN pixels")?;
                                packed >> 4
                            } else {
                                packed & 15
                            }
                        };
                        if x < 0 || x as usize + pixel >= width || y >= height {
                            return Err(format!("{name}: SCN pixel outside frame {frame}"));
                        }
                        indices[y * width + x as usize + pixel] =
                            Some(color.checked_add(base).ok_or("Invalid SCN palette index")?);
                    }
                    x += count as i32;
                }
                _ => unreachable!(),
            }
            if x < 0 || x as usize > width {
                return Err(format!("{name}: SCN cursor outside frame {frame}"));
            }
            last = command;
        }
        let rgba = indices
            .into_iter()
            .flat_map(|index| match index {
                Some(i) => {
                    let p = &palette[i as usize * 3..i as usize * 3 + 3];
                    [
                        vga_channel(p[0]),
                        vga_channel(p[1]),
                        vga_channel(p[2]),
                        if i == 0 { 0 } else { 255 },
                    ]
                }
                None => [0, 0, 0, 0],
            })
            .collect();
        frames.push(Screen {
            width: width as u32,
            height: height as u32,
            rgba,
        });
    }
    Ok(frames)
}

/// Expand a 6-bit VGA DAC channel to 8-bit RGB by replicating its high bits.
/// This matches the values exposed by VGA-compatible adapters and DOSBox.
fn vga_channel(channel: u8) -> u8 {
    (channel << 2) | (channel >> 4)
}

pub fn palette_color(resources: &Resources, name: &str, index: u8) -> Result<[u8; 4]> {
    let mut list = Vec::new();
    chunks(resources.get(name)?, &mut list)?;
    let palette = list
        .iter()
        .find(|(tag, _)| *tag == b"VGA:")
        .ok_or("Missing VGA palette")?
        .1;
    let p = palette
        .get(index as usize * 3..index as usize * 3 + 3)
        .ok_or("Missing palette entry")?;
    Ok([vga_channel(p[0]), vga_channel(p[1]), vga_channel(p[2]), 255])
}

/// Method 3: LZSS with adaptive Huffman coding (Okumura's LZHUF). The EXE
/// carries the matching encoder: initialize_huffman_tree (2c25:1e63) builds
/// 314 leaves (F 60, threshold 2) in a 627-node tree, and emit_huffman_field
/// (2c25:2257) writes match positions with the 64-entry tables at DS:3606.
fn lzhuf(input: &[u8], expected: usize) -> Result<Vec<u8>> {
    const N: usize = 4096;
    const F: usize = 60;
    const THRESHOLD: usize = 2;
    const N_CHAR: usize = 256 - THRESHOLD + F;
    const T: usize = N_CHAR * 2 - 1;
    const R: usize = T - 1;
    const MAX_FREQ: u16 = 0x8000;
    // DS:3606 (code lengths) and DS:3646 (codes) of the upper six bits.
    let mut d_code = [0u8; 256];
    let mut d_len = [0u8; 256];
    let mut code = 0usize;
    for (index, (len, count)) in [(3u8, 1usize), (4, 3), (5, 8), (6, 12), (7, 24), (8, 16)]
        .into_iter()
        .flat_map(|(len, n)| std::iter::repeat_n(len, n).map(move |l| (l, 1usize << (8 - len))))
        .enumerate()
    {
        d_code[code..code + count].fill(index as u8);
        d_len[code..code + count].fill(len);
        code += count;
    }
    let mut freq = [0u16; T + 1];
    let mut parent = [0usize; T + N_CHAR];
    let mut son = [0usize; T];
    for i in 0..N_CHAR {
        freq[i] = 1;
        son[i] = i + T;
        parent[i + T] = i;
    }
    let (mut i, mut j) = (0, N_CHAR);
    while j < T {
        freq[j] = freq[i] + freq[i + 1];
        son[j] = i;
        parent[i] = j;
        parent[i + 1] = j;
        i += 2;
        j += 1;
    }
    freq[T] = 0xffff;
    parent[R] = 0;

    let (mut position, mut buffer, mut length) = (0usize, 0u16, 0u32);
    let fill = |position: &mut usize, buffer: &mut u16, length: &mut u32| {
        while *length <= 8 {
            let byte = input.get(*position).copied().unwrap_or(0);
            *position += 1;
            *buffer |= u16::from(byte) << (8 - *length);
            *length += 8;
        }
    };
    let bits = |count: u32, position: &mut usize, buffer: &mut u16, length: &mut u32| {
        fill(position, buffer, length);
        let value = *buffer >> (16 - count);
        *buffer = if count == 16 { 0 } else { *buffer << count };
        *length -= count;
        usize::from(value)
    };

    let mut text = [0x20u8; N];
    let mut r = N - F;
    let mut output = Vec::with_capacity(expected);
    while output.len() < expected {
        if position > input.len() + 2 {
            return Err("Truncated LZHUF stream".into());
        }
        let mut c = son[R];
        while c < T {
            c = son[c + bits(1, &mut position, &mut buffer, &mut length)];
        }
        c -= T;
        // update: rebuild when the root saturates, then bubble the leaf up.
        if freq[R] == MAX_FREQ {
            let mut k = 0;
            for i in 0..T {
                if son[i] >= T {
                    freq[k] = freq[i].div_ceil(2);
                    son[k] = son[i];
                    k += 1;
                }
            }
            let mut i = 0;
            for j in N_CHAR..T {
                let f = freq[i] + freq[i + 1];
                let mut k = j - 1;
                while f < freq[k] {
                    k -= 1;
                }
                k += 1;
                freq.copy_within(k..j, k + 1);
                freq[k] = f;
                son.copy_within(k..j, k + 1);
                son[k] = i;
                i += 2;
            }
            for i in 0..T {
                let k = son[i];
                parent[k] = i;
                if k < T {
                    parent[k + 1] = i;
                }
            }
        }
        let mut node = parent[c + T];
        loop {
            freq[node] += 1;
            let k = freq[node];
            let mut l = node + 1;
            if k > freq[l] {
                while k > freq[l + 1] {
                    l += 1;
                }
                freq[node] = freq[l];
                freq[l] = k;
                let i = son[node];
                parent[i] = l;
                if i < T {
                    parent[i + 1] = l;
                }
                let j = son[l];
                son[l] = i;
                parent[j] = node;
                if j < T {
                    parent[j + 1] = node;
                }
                son[node] = j;
                node = l;
            }
            node = parent[node];
            if node == 0 {
                break;
            }
        }
        if c < 256 {
            output.push(c as u8);
            text[r] = c as u8;
            r = (r + 1) & (N - 1);
        } else {
            let first = bits(8, &mut position, &mut buffer, &mut length);
            let mut low = first;
            for _ in 0..d_len[first] - 2 {
                low = (low << 1) + bits(1, &mut position, &mut buffer, &mut length);
            }
            let distance = (usize::from(d_code[first]) << 6) | (low & 0x3f);
            let start = (r + N - distance - 1) & (N - 1);
            for k in 0..c - 255 + THRESHOLD {
                let byte = text[(start + k) & (N - 1)];
                output.push(byte);
                text[r] = byte;
                r = (r + 1) & (N - 1);
            }
        }
    }
    output.truncate(expected);
    Ok(output)
}
