use crate::ids::FileId;

pub struct SourceFile {
    pub id: FileId,
    pub name: String,
    pub source: String,
    line_starts: Vec<u32>,
}

impl SourceFile {
    pub fn new(id: FileId, name: String, source: String) -> Self {
        let mut line_starts = vec![0];
        for (i, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push((i + 1) as u32);
            }
        }

        Self {
            id,
            name,
            source,
            line_starts,
        }
    }

    /// Returns (line, Unicode scalar column), 1-indexed. Spans remain byte-based.
    pub fn get_line_col(&self, byte_offset: u32) -> (u32, u32) {
        let mut byte_offset = (byte_offset as usize).min(self.source.len());
        while !self.source.is_char_boundary(byte_offset) {
            byte_offset -= 1;
        }
        let byte_offset = byte_offset as u32;
        let line_idx = match self.line_starts.binary_search(&byte_offset) {
            Ok(idx) => idx,
            Err(idx) => idx - 1,
        };
        let line_start = self.line_starts[line_idx];
        let col = self.source[line_start as usize..byte_offset as usize].chars().count();
        (line_idx as u32 + 1, col as u32 + 1)
    }

    pub fn get_line_str(&self, line_idx: u32) -> Option<&str> {
        let idx = (line_idx - 1) as usize;
        if idx >= self.line_starts.len() {
            return None;
        }
        let start = self.line_starts[idx] as usize;
        let end = if idx + 1 < self.line_starts.len() {
            self.line_starts[idx + 1] as usize - 1 // Exclude \n
        } else {
            self.source.len()
        };

        // Handle \r if on windows
        let mut end = end;
        if end > start && self.source.as_bytes()[end - 1] == b'\r' {
            end -= 1;
        }

        self.source.get(start..end)
    }
}

#[derive(Default)]
pub struct SourceManager {
    files: Vec<SourceFile>,
}

impl SourceManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(&mut self, name: String, source: String) -> FileId {
        let id = FileId(self.files.len() as u32);
        self.files.push(SourceFile::new(id, name, source));
        id
    }

        pub fn append_to_file(&mut self, id: FileId, extra: &str) {
        if let Some(file) = self.files.get_mut(id.0 as usize) {
            let offset = file.source.len();
            file.source.push_str(extra);
            for (i, byte) in extra.bytes().enumerate() {
                if byte == b'\n' {
                    file.line_starts.push((offset + i + 1) as u32);
                }
            }
        }
    }
    pub fn get_file(&self, id: FileId) -> Option<&SourceFile> {
        self.files.get(id.0 as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_col() {
        let mut sm = SourceManager::new();
        let fid = sm.add_file("test.ms".to_string(), "hello\nworld\n!".to_string());

        let file = sm.get_file(fid).unwrap();

        assert_eq!(file.get_line_col(0), (1, 1)); // 'h'
        assert_eq!(file.get_line_col(5), (1, 6)); // '\n'
        assert_eq!(file.get_line_col(6), (2, 1)); // 'w'

        assert_eq!(file.get_line_str(2), Some("world"));
    }

    #[test]
    fn columns_count_unicode_scalars_while_spans_use_bytes() {
        let file = SourceFile::new(FileId(0), "unicode.ln".into(), "é漢\r\nnope".into());
        assert_eq!(file.get_line_col(5), (1, 3));
        assert_eq!(file.get_line_col(7), (2, 1));
        assert_eq!(file.get_line_str(1), Some("é漢"));
        // Malformed/out-of-range diagnostic offsets must not panic in rendering.
        assert_eq!(file.get_line_col(1), (1, 1));
        assert_eq!(file.get_line_col(999), (2, 5));
    }
}
