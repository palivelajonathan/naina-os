//! Read-only Markdown vault indexer for NAINA OS.

use crate::error::{MemoryError, Result};
use std::fs;
use std::path::Path;

/// Information parsed from a Markdown document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMarkdown {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub file_path: String,
}

/// Helper utility for reading and parsing Markdown files.
pub struct VaultIndexer;

impl VaultIndexer {
    /// Recursively discovers and parses all `.md` files under the specified directory path.
    ///
    /// Files are opened strictly read-only. Source files are never modified.
    pub fn scan_directory(dir_path: impl AsRef<Path>) -> Result<Vec<ParsedMarkdown>> {
        let path = dir_path.as_ref();
        if !path.exists() || !path.is_dir() {
            return Err(MemoryError::VaultNotFound {
                path: path.display().to_string(),
            });
        }

        let mut results = Vec::new();
        Self::traverse_dir(path, &mut results)?;
        Ok(results)
    }

    fn traverse_dir(dir: &Path, results: &mut Vec<ParsedMarkdown>) -> Result<()> {
        let entries = fs::read_dir(dir)?;

        for entry_res in entries {
            let entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            if path.is_dir() {
                let _ = Self::traverse_dir(&path, results);
            } else if path.is_file()
                && path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("md"))
                    .unwrap_or(false)
            {
                let parse_res = Self::parse_file(&path);
                if let Ok(parsed) = parse_res {
                    results.push(parsed);
                }
            }
        }

        Ok(())
    }

    /// Parses a single Markdown file read-only into a [`ParsedMarkdown`].
    pub fn parse_file(file_path: &Path) -> Result<ParsedMarkdown> {
        let content = fs::read_to_string(file_path)?;
        let path_str = file_path.to_string_lossy().to_string();

        let title = Self::extract_title(&content, file_path);
        let tags = Self::extract_tags(&content);

        Ok(ParsedMarkdown {
            title,
            content,
            tags,
            file_path: path_str,
        })
    }

    fn extract_title(content: &str, file_path: &Path) -> String {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(stripped) = trimmed.strip_prefix("# ") {
                let title = stripped.trim();
                if !title.is_empty() {
                    return title.to_string();
                }
            }
        }

        file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled Document")
            .to_string()
    }

    fn extract_tags(content: &str) -> Vec<String> {
        let mut tags = Vec::new();

        for word in content.split_whitespace() {
            if word.starts_with('#') && word.len() > 1 {
                let tag = word[1..]
                    .trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
                    .to_lowercase();
                if !tag.is_empty() && !tags.contains(&tag) {
                    tags.push(tag);
                }
            }
        }

        tags
    }
}
