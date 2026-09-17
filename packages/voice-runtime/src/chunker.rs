//! Text Chunker for incremental Token-to-Speech Streaming in NAINA OS.
//!
//! Provides [`TextChunker`], an incremental buffer that accumulates LLM token strings
//! and emits synthesis-safe prosodic text chunks to TTS engines (e.g. Piper) as early
//! as possible while preserving exact text reconstruction.

/// Incremental text chunker that splits a token stream into synthesis-safe prosodic units.
#[derive(Debug, Clone)]
pub struct TextChunker {
    buffer: String,
    min_clause_chars: usize,
    max_chunk_chars: usize,
}

impl Default for TextChunker {
    fn default() -> Self {
        Self::new()
    }
}

impl TextChunker {
    /// Constructs a new [`TextChunker`] with default boundaries:
    /// - `min_clause_chars`: 8 (minimum characters required before splitting on clause punctuation `, ; :`)
    /// - `max_chunk_chars`: 60 (maximum characters allowed before splitting at the nearest whitespace)
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            min_clause_chars: 8,
            max_chunk_chars: 60,
        }
    }

    /// Constructs a [`TextChunker`] with custom limits.
    pub fn with_limits(min_clause_chars: usize, max_chunk_chars: usize) -> Self {
        Self {
            buffer: String::new(),
            min_clause_chars,
            max_chunk_chars,
        }
    }

    /// Feeds an incremental token or text fragment into the chunker.
    /// Returns a list of any newly completed synthesis-safe text chunks.
    pub fn push(&mut self, token: &str) -> Vec<String> {
        self.buffer.push_str(token);
        let mut chunks = Vec::new();

        loop {
            if let Some(split_idx) = self.find_split_point() {
                // Extract chunk up to split_idx, preserving all characters exactly
                let chunk: String = self.buffer.drain(..split_idx).collect();
                chunks.push(chunk);
            } else {
                break;
            }
        }

        chunks
    }

    /// Finds the split point byte index in the current buffer, if a synthesis boundary is reached.
    fn find_split_point(&self) -> Option<usize> {
        if self.buffer.trim().is_empty() {
            return None;
        }

        // 1. Scan in sequential order for earliest sentence boundary (. ! ? \n) or clause boundary (, ; : —)
        for (i, c) in self.buffer.char_indices() {
            if c == '.' || c == '!' || c == '?' || c == '\n' {
                if !self.buffer[..i].trim().is_empty() {
                    return Some(i + c.len_utf8());
                }
            } else if (c == ',' || c == ';' || c == ':' || c == '—' || c == '-')
                && i >= self.min_clause_chars
            {
                if !self.buffer[..i].trim().is_empty() {
                    return Some(i + c.len_utf8());
                }
            }
        }

        // 2. Look for whitespace boundary if buffer exceeds max_chunk_chars
        if self.buffer.len() >= self.max_chunk_chars {
            if let Some((i, c)) = self
                .buffer
                .char_indices()
                .filter(|(idx, ch)| ch.is_whitespace() && *idx >= self.min_clause_chars)
                .last()
            {
                if !self.buffer[..i].trim().is_empty() {
                    return Some(i + c.len_utf8());
                }
            }
        }

        None
    }

    /// Flushes any remaining partial chunk when the token stream finishes.
    pub fn flush(&mut self) -> Option<String> {
        if self.buffer.trim().is_empty() {
            self.buffer.clear();
            None
        } else {
            let remaining = std::mem::take(&mut self.buffer);
            Some(remaining)
        }
    }

    /// Returns the current contents of the internal unchunked buffer.
    pub fn current_buffer(&self) -> &str {
        &self.buffer
    }

    /// Clears the internal buffer.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentence_boundary_chunking() {
        let mut chunker = TextChunker::new();
        let tokens = vec![
            "Hello", "!", " How", " are", " you", "?", " I", " am", " fine", ".",
        ];

        let mut all_chunks = Vec::new();
        for t in &tokens {
            all_chunks.extend(chunker.push(t));
        }
        if let Some(final_chunk) = chunker.flush() {
            all_chunks.push(final_chunk);
        }

        assert_eq!(all_chunks.len(), 3);
        assert_eq!(all_chunks[0], "Hello!");
        assert_eq!(all_chunks[1], " How are you?");
        assert_eq!(all_chunks[2], " I am fine.");

        // Exact text reconstruction invariant
        assert_eq!(all_chunks.join(""), tokens.join(""));
    }

    #[test]
    fn test_clause_comma_chunking() {
        let mut chunker = TextChunker::with_limits(10, 50);
        let tokens = vec!["Good", " morning", ",", " let", " us", " begin", "."];

        let mut all_chunks = Vec::new();
        for t in &tokens {
            all_chunks.extend(chunker.push(t));
        }
        if let Some(final_chunk) = chunker.flush() {
            all_chunks.push(final_chunk);
        }

        assert_eq!(all_chunks.len(), 2);
        assert_eq!(all_chunks[0], "Good morning,");
        assert_eq!(all_chunks[1], " let us begin.");
        assert_eq!(all_chunks.join(""), tokens.join(""));
    }

    #[test]
    fn test_whitespace_overflow_chunking() {
        let mut chunker = TextChunker::with_limits(5, 20);
        let tokens = vec![
            "This",
            " is",
            " a",
            " long",
            " sentence",
            " without",
            " punctuation",
            " at",
            " all",
        ];

        let mut all_chunks = Vec::new();
        for t in &tokens {
            all_chunks.extend(chunker.push(t));
        }
        if let Some(final_chunk) = chunker.flush() {
            all_chunks.push(final_chunk);
        }

        assert!(all_chunks.len() >= 2);
        assert_eq!(all_chunks.join(""), tokens.join(""));
    }

    #[test]
    fn test_exact_reconstruction_character_by_character() {
        let text = "Hello! I'm doing well, thank you. How can I assist you today?";
        let mut chunker = TextChunker::new();
        let mut all_chunks = Vec::new();

        for ch in text.chars() {
            let mut buf = [0u8; 4];
            let s = ch.encode_utf8(&mut buf);
            all_chunks.extend(chunker.push(s));
        }
        if let Some(final_chunk) = chunker.flush() {
            all_chunks.push(final_chunk);
        }

        assert_eq!(all_chunks.join(""), text);
    }
}
