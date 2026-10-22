//! Version 2 business-resource archive: independent 64 KiB Deflate blocks.
//! Index JSON is [2, {path: [raw_length, [[compressed_offset, compressed_length], ...]]}].
use anyhow::{Result, anyhow, ensure};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
};

pub const BLOCK_BYTES: usize = 64 * 1024;
pub type Entry = (usize, Vec<(usize, usize)>);
pub type Index = BTreeMap<String, Entry>;

pub fn append(data: &mut Vec<u8>, bytes: &[u8]) -> Result<Entry> {
    let mut blocks = Vec::new();
    for block in bytes.chunks(BLOCK_BYTES) {
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(block)?;
        let compressed = encoder.finish()?;
        blocks.push((data.len(), compressed.len()));
        data.extend(compressed);
    }
    Ok((bytes.len(), blocks))
}

pub fn encode_index(index: &Index) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&(2u32, index))?)
}

pub fn decode_index(bytes: &[u8], data_len: usize) -> Result<Index> {
    let (version, index): (u32, Index) = serde_json::from_slice(bytes)?;
    ensure!(
        version == 2,
        "unsupported resource archive version; rebuild the application"
    );
    let mut ranges = Vec::new();
    for (size, blocks) in index.values() {
        ensure!(
            blocks.len() == size.div_ceil(BLOCK_BYTES),
            "invalid resource block count"
        );
        for &(offset, length) in blocks {
            let end = offset
                .checked_add(length)
                .ok_or_else(|| anyhow!("resource range overflow"))?;
            ensure!(
                length > 0 && end <= data_len,
                "resource block is out of bounds"
            );
            ranges.push((offset, end));
        }
    }
    ranges.sort_unstable();
    let mut cursor = 0;
    for (offset, end) in ranges {
        ensure!(offset == cursor, "resource blocks overlap or leave a gap");
        cursor = end;
    }
    ensure!(cursor == data_len, "unindexed resource data");
    Ok(index)
}

pub fn read_range(data: &[u8], entry: &Entry, offset: usize, length: usize) -> Result<Vec<u8>> {
    ensure!(offset <= entry.0, "invalid resource range");
    let length = length.min(entry.0 - offset);
    let end = offset + length;
    let mut output = Vec::with_capacity(length);
    if length == 0 {
        return Ok(output);
    }
    for block_id in offset / BLOCK_BYTES..=(end - 1) / BLOCK_BYTES {
        let &(start, compressed_len) = entry
            .1
            .get(block_id)
            .ok_or_else(|| anyhow!("missing resource block"))?;
        let compressed_end = start
            .checked_add(compressed_len)
            .ok_or_else(|| anyhow!("resource range overflow"))?;
        let compressed = data
            .get(start..compressed_end)
            .ok_or_else(|| anyhow!("truncated resource block"))?;
        let expected = BLOCK_BYTES.min(entry.0 - block_id * BLOCK_BYTES);
        let mut decoder = flate2::read::DeflateDecoder::new(compressed);
        let mut raw = Vec::with_capacity(expected);
        (&mut decoder)
            .take((expected + 1) as u64)
            .read_to_end(&mut raw)?;
        ensure!(
            raw.len() == expected && decoder.total_in() == compressed_len as u64,
            "invalid resource block size"
        );
        let from = offset.saturating_sub(block_id * BLOCK_BYTES);
        let to = expected.min(end - block_id * BLOCK_BYTES);
        output.extend_from_slice(&raw[from..to]);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges_decode_only_intersecting_blocks() {
        let input: Vec<_> = (0..BLOCK_BYTES * 3 + 7).map(|i| (i % 251) as u8).collect();
        let mut data = Vec::new();
        let entry = append(&mut data, &input).unwrap();
        // Damage an unrelated prefix block. A tail read must not decode it.
        data[..entry.1[0].1].fill(255);
        assert_eq!(
            read_range(&data, &entry, BLOCK_BYTES * 2 - 3, 19).unwrap(),
            input[BLOCK_BYTES * 2 - 3..BLOCK_BYTES * 2 + 16]
        );
        assert!(read_range(&data, &entry, 0, 1).is_err());
    }
    #[test]
    fn index_rejects_old_overlapping_and_truncated_archives() {
        assert!(decode_index(br#"{"a":[0,3]}"#, 3).is_err());
        assert!(decode_index(br#"[2,{"a":[1,[[0,2]]],"b":[1,[[0,2]]]}]"#, 2).is_err());
        assert!(decode_index(br#"[2,{"a":[1,[[0,3]]]}]"#, 2).is_err());
        let mut data = Vec::new();
        let entry = append(&mut data, b"hello").unwrap();
        let index = Index::from([("a".into(), entry.clone())]);
        assert_eq!(
            decode_index(&encode_index(&index).unwrap(), data.len()).unwrap(),
            index
        );
        assert_eq!(read_range(&data, &entry, 4, 20).unwrap(), b"o");
    }
}
