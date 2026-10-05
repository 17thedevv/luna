//! Mutate an artifact section while retaining a coherent section table.
use luna_llib::{LlibHeader, SectionEntry, SectionType};
use std::io::{Cursor, Seek, SeekFrom};

pub fn rewrite(bytes: &[u8], mut change: impl FnMut(SectionType, &mut Vec<u8>)) -> Vec<u8> {
    let mut input = Cursor::new(bytes);
    let header = LlibHeader::read_from(&mut input).unwrap();
    input
        .seek(SeekFrom::Start(header.section_table_offset))
        .unwrap();
    let mut entries = Vec::new();
    for _ in 0..header.section_count {
        entries.push(SectionEntry::read_from(&mut input).unwrap());
    }
    let mut offset = input.position();
    let mut payloads = Vec::new();
    for entry in &mut entries {
        let mut data = bytes[entry.offset as usize..(entry.offset + entry.size) as usize].to_vec();
        change(entry.section_type, &mut data);
        entry.offset = offset;
        entry.size = data.len() as u64;
        entry.hash = luna_llib::section_checksum(&data);
        offset += entry.size;
        payloads.push(data);
    }
    let mut output = Vec::new();
    header.write_to(&mut output).unwrap();
    assert_eq!(output.len() as u64, header.section_table_offset);
    for entry in entries {
        entry.write_to(&mut output).unwrap();
    }
    for payload in payloads {
        output.extend(payload);
    }
    output
}

pub fn change_manifest(bytes: &[u8], change: impl Fn(&mut luna_llib::Manifest)) -> Vec<u8> {
    rewrite(bytes, |kind, payload| {
        if kind == SectionType::Manifest {
            let mut manifest = bincode::deserialize(payload).unwrap();
            change(&mut manifest);
            *payload = bincode::serialize(&manifest).unwrap();
        }
    })
}
