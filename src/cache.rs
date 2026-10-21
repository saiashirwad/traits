//! Content-addressed `check_traits` assessments. A key hashes everything one
//! decision question sends, so one cache directory serves every repository and
//! worktree.

use std::path::{Path, PathBuf};

use crate::doc::Assessment;

/// The hash of one question's provider identity, input, and question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key(blake3::Hash);

/// A source's provider identity and input, hashed once and extended by each
/// question.
pub struct Keys(blake3::Hasher);

impl Keys {
    pub fn new(provider: &[u8], input: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        framed(&mut hasher, provider);
        framed(&mut hasher, input);
        Self(hasher)
    }

    pub fn key(&self, question: &[u8]) -> Key {
        let mut hasher = self.0.clone();
        framed(&mut hasher, question);
        Key(hasher.finalize())
    }
}

fn framed(hasher: &mut blake3::Hasher, part: &[u8]) {
    hasher.update(&(part.len() as u64).to_le_bytes());
    hasher.update(part);
}

fn entry(dir: &Path, key: Key) -> PathBuf {
    dir.join(format!("{}.json", key.0.to_hex()))
}

/// A missing or unreadable entry is a miss. Reads block: entries are tiny,
/// and a blocking read measured faster than any async or parallel one.
pub fn read(dir: &Path, key: Key) -> Option<Assessment> {
    let bytes = std::fs::read(entry(dir, key)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Store an assessment through a temporary file, so readers never see a
/// partial entry. Failures only cost a future cache miss.
pub async fn write(dir: &Path, key: Key, assessment: &Assessment) {
    let path = entry(dir, key);
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = async {
        tokio::fs::create_dir_all(dir).await?;
        let bytes = serde_json::to_vec(assessment).map_err(std::io::Error::other)?;
        tokio::fs::write(&temporary, bytes).await?;
        tokio::fs::rename(&temporary, &path).await
    }
    .await;
    if let Err(error) = result {
        let _ = tokio::fs::remove_file(&temporary).await;
        tracing::warn!(path = %path.display(), %error, "failed to cache an assessment");
    }
}
