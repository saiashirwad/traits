use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;

use crate::doc::{Trait, TraitName};
use crate::error::ToolFailure;

#[async_trait]
pub trait TraitStore: Send + Sync {
    async fn ls(&self) -> Result<Vec<Trait>, ToolFailure>;
    async fn get(&self, name: &TraitName) -> Result<Trait, ToolFailure>;
    async fn add(&self, trait_value: Trait) -> Result<Trait, ToolFailure>;
    async fn update(&self, trait_value: Trait) -> Result<(), ToolFailure>;
    async fn rm(&self, names: &[TraitName]) -> Result<(), ToolFailure>;
}

pub struct FileSystemStore {
    root: PathBuf,
}

impl FileSystemStore {
    pub async fn open(root: PathBuf) -> Result<Self, ToolFailure> {
        let root = if root.is_absolute() {
            root
        } else {
            let cwd = std::env::current_dir().map_err(|error| {
                ToolFailure::platform(format!("Unable to resolve the trait store path: {error}"))
            })?;
            cwd.join(root)
        };
        tokio::fs::create_dir_all(&root).await.map_err(|error| {
            ToolFailure::platform(format!(
                "Unable to create the trait store at {}: {error}",
                root.display()
            ))
        })?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn trait_path(&self, name: &str) -> PathBuf {
        self.root.join(format!("{name}.md"))
    }

    fn display(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    /// Parse and validate the contents of a trait file, enforcing the filename invariant.
    fn validate(&self, text: &str, path: &Path) -> Result<Trait, ToolFailure> {
        let path_text = Self::display(path);
        let trait_value = crate::doc::parse(text, &path_text)?;
        let expected = self.trait_path(trait_value.name.as_str());
        if expected != path {
            return Err(ToolFailure::filename_mismatch(
                &path_text,
                &format!("{}.md", trait_value.name.as_str()),
            ));
        }
        Ok(trait_value)
    }

    /// Read a trait file. `not_found` is the trait name reported when the file is absent.
    async fn read_file(&self, path: &Path, not_found: &str) -> Result<Trait, ToolFailure> {
        let text = match tokio::fs::read_to_string(path).await {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(ToolFailure::not_found(not_found));
            }
            Err(error) => {
                return Err(ToolFailure::platform(format!(
                    "Unable to read {}: {error}",
                    path.display()
                )));
            }
        };
        self.validate(&text, path)
    }
}

#[async_trait]
impl TraitStore for FileSystemStore {
    async fn ls(&self) -> Result<Vec<Trait>, ToolFailure> {
        let list_error = |error: std::io::Error| {
            ToolFailure::platform(format!("Unable to list {}: {error}", self.root.display()))
        };
        let mut dir = tokio::fs::read_dir(&self.root).await.map_err(list_error)?;
        let mut names: Vec<String> = Vec::new();
        while let Some(entry) = dir.next_entry().await.map_err(list_error)? {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
        names.sort();

        let mut traits = Vec::new();
        for name in names {
            if !name.ends_with(".md") {
                continue;
            }
            let path = self.root.join(&name);
            let metadata = tokio::fs::metadata(&path).await.map_err(|error| {
                ToolFailure::platform(format!("Unable to inspect {}: {error}", path.display()))
            })?;
            if !metadata.is_file() {
                continue;
            }
            let text = tokio::fs::read_to_string(&path).await.map_err(|error| {
                ToolFailure::platform(format!("Unable to read {}: {error}", path.display()))
            })?;
            traits.push(self.validate(&text, &path)?);
        }
        Ok(traits)
    }

    async fn get(&self, name: &TraitName) -> Result<Trait, ToolFailure> {
        self.read_file(&self.trait_path(name.as_str()), name.as_str())
            .await
    }

    async fn add(&self, trait_value: Trait) -> Result<Trait, ToolFailure> {
        let path = self.trait_path(trait_value.name.as_str());
        let contents = crate::doc::serialize(&trait_value);
        let mut file = match tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(ToolFailure::already_exists(trait_value.name.as_str()));
            }
            Err(error) => {
                return Err(ToolFailure::platform(format!(
                    "Unable to write {}: {error}",
                    path.display()
                )));
            }
        };
        file.write_all(contents.as_bytes()).await.map_err(|error| {
            ToolFailure::platform(format!("Unable to write {}: {error}", path.display()))
        })?;
        file.flush().await.map_err(|error| {
            ToolFailure::platform(format!("Unable to write {}: {error}", path.display()))
        })?;
        Ok(trait_value)
    }

    /// Replace an existing trait by renaming a fully written, synced sibling
    /// over the file it resolves to, so readers see the old contents or the
    /// new ones, never a partial write. Symlinks and permissions are kept.
    async fn update(&self, trait_value: Trait) -> Result<(), ToolFailure> {
        let path = self.trait_path(trait_value.name.as_str());
        self.get(&trait_value.name).await?;
        let failed = |error: std::io::Error| {
            ToolFailure::platform(format!("Unable to write {}: {error}", path.display()))
        };
        let target = tokio::fs::canonicalize(&path).await.map_err(failed)?;
        let staged = target.with_file_name(format!(
            ".{}.{}.tmp",
            trait_value.name.as_str(),
            uuid::Uuid::new_v4()
        ));
        let replaced = async {
            let permissions = tokio::fs::metadata(&target).await?.permissions();
            let mut file = tokio::fs::File::create(&staged).await?;
            file.write_all(crate::doc::serialize(&trait_value).as_bytes())
                .await?;
            file.sync_all().await?;
            tokio::fs::set_permissions(&staged, permissions).await?;
            tokio::fs::rename(&staged, &target).await
        }
        .await;
        if let Err(error) = replaced {
            let _ = tokio::fs::remove_file(&staged).await;
            return Err(failed(error));
        }
        Ok(())
    }

    async fn rm(&self, names: &[TraitName]) -> Result<(), ToolFailure> {
        for name in names {
            let trait_value = self.get(name).await?;
            let path = self.trait_path(trait_value.name.as_str());
            tokio::fs::remove_file(&path).await.map_err(|error| {
                ToolFailure::platform(format!("Unable to remove {}: {error}", path.display()))
            })?;
        }
        Ok(())
    }
}
