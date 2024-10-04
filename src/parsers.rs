use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use anyhow::{anyhow, Context};
use image::{codecs::bmp::BmpDecoder, ColorType, ImageDecoder};

pub struct MapImage {
    width: u32,
    height: u32,
    bytes: Box<[u8]>,
}
fn parse_region_image(path: &Path) -> anyhow::Result<MapImage> {
    let file = File::open(path)?;
    let decoder = BmpDecoder::new(BufReader::new(file))?;
    let (width, height) = decoder.dimensions();
    if decoder.color_type() != ColorType::Rgb8 {
        return Err(anyhow!("image format of file should be RGB8"));
    }
    let mut bytes: Box<[u8]> = (0..(width * height * 3)).map(|_| 0).collect();
    decoder.read_image(&mut *bytes)?;
    Ok(MapImage {
        width,
        height,
        bytes,
    })
}

pub fn parse_province_definitions(path: &Path) -> anyhow::Result<HashMap<[u8; 3], u32>> {
    let mut file = String::new();
    File::open(path)?.read_to_string(&mut file)?;
    file.lines()
        .enumerate()
        .map(|(idx, line)| {
            let mut iter = line.split(";");
            let id = iter
                .next()
                .with_context(|| format!("Not enough elements on line {}", idx))?
                .parse()?;
            let mut parse_u8 = || -> anyhow::Result<u8> {
                let str = iter
                    .next()
                    .with_context(|| format!("Not enough elements on line {}", idx))?;
                u8::from_str_radix(str, 10).map_err(Into::into)
            };
            let rgb = [parse_u8()?, parse_u8()?, parse_u8()?];
            Ok((rgb, id))
        })
        .collect()
}

pub fn parse_state_provinces(path: &Path) -> anyhow::Result<Vec<u32>> {
    let mut buf = String::new();
    File::open(path)?.read_to_string(&mut buf)?;
    let mut curr = &*buf;
    let mut trimmed = String::new();
    while let Some(idx) = curr.find('#') {
        trimmed.push_str(&curr[0..idx]);
        curr = &curr[idx..];
        let Some(idx) = curr.find('\n') else { break };
        curr = &curr[idx..]
    }
    trimmed.push_str(curr);

    // (rem, within)
    fn block(mut input: &str) -> anyhow::Result<(&str, &str)> {
        if !input.starts_with('{') {
            return Err(anyhow!("block doesn't start with {{"));
        }
        input = &input[1..];
        let mut height = 1;
        for (idx, char) in input.char_indices() {
            if char == '{' {
                height += 1;
            }
            if char == '}' {
                height -= 1;
                if height == 0 {
                    return Ok((&input[(idx + 1)..], &input[0..idx]));
                }
            }
        }
        Err(anyhow!(
            "missing {height} }} characters at the end of the block, {input}"
        ))
    }

    fn parse_kv(mut input: &str) -> anyhow::Result<HashMap<&str, &str>> {
        let mut acc = HashMap::new();
        loop {
            input = input.trim_start();
            if input.is_empty() {
                break;
            }
            let end_key = input.find('=').context("key doesn't have a = afterwards")?;
            let key = input[0..end_key].trim_end();
            if key.is_empty() {
                return Err(anyhow!("empty key"));
            }
            input = input[(end_key + 1)..].trim_start();
            let value = if input.starts_with('{') {
                let (rem, value) = block(input)?;
                input = rem;
                value
            } else {
                let value_end = input
                    .find(|c: char| c.is_ascii_whitespace())
                    .unwrap_or(input.len());
                let value = &input[0..value_end];
                input = &input[value_end..];
                value
            };
            if let Some(e) = acc.insert(key, value) {
                return Err(anyhow!(
                    "key {key} has two entries with values: {e} and {value}"
                ));
            }
        }
        Ok(acc)
    }

    fn parse_array_u32(mut input: &str) -> anyhow::Result<Vec<u32>> {
        let mut acc = Vec::new();
        loop {
            input = input.trim_start();
            if input.is_empty() {
                break;
            }
            let end_curr = input
                .find(|c: char| c.is_ascii_whitespace())
                .unwrap_or(input.len());
            acc.push(input[0..end_curr].parse().context("invalid u32")?);
            input = &input[end_curr..];
        }
        Ok(acc)
    }

    let in_block = *parse_kv(&*trimmed)?
        .get("state")
        .context("no state field")?;
    let in_provinces = *parse_kv(in_block)?
        .get("provinces")
        .context("no provinces field")?;
    parse_array_u32(in_provinces)
}
