use async_trait::async_trait;

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
    root: std::path::PathBuf,
}

impl FileSystemStore {
    pub async fn open(root: std::path::PathBuf) -> Result<Self, ToolFailure> {
        let _ = &root;
        todo!("delegate: docstore")
    }

    pub fn root(&self) -> &std::path::Path {
        &self.root
    }
}

#[async_trait]
impl TraitStore for FileSystemStore {
    async fn ls(&self) -> Result<Vec<Trait>, ToolFailure> {
        todo!("delegate: docstore")
    }

    async fn get(&self, name: &TraitName) -> Result<Trait, ToolFailure> {
        let _ = name;
        todo!("delegate: docstore")
    }

    async fn add(&self, trait_value: Trait) -> Result<Trait, ToolFailure> {
        let _ = trait_value;
        todo!("delegate: docstore")
    }

    async fn update(&self, trait_value: Trait) -> Result<(), ToolFailure> {
        let _ = trait_value;
        todo!("delegate: docstore")
    }

    async fn rm(&self, names: &[TraitName]) -> Result<(), ToolFailure> {
        let _ = names;
        todo!("delegate: docstore")
    }
}
