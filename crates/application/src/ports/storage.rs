//! 业务记录与素材存取接口；接口按用例定义，不建设通用 CRUD 基类。

use meowlive_domain::resources::{ReferenceAsset, ReferenceAudioMetadata, ResourceCatalog};
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredReference {
    pub reference: ReferenceAsset,
    pub audio: ReferenceAudioMetadata,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceStoreError {
    pub message: String,
}

impl ResourceStoreError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ResourceStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ResourceStoreError {}

pub trait ResourceStore: Send + Sync {
    fn load(&self) -> Result<ResourceCatalog, ResourceStoreError>;
    fn save(&self, catalog: &ResourceCatalog) -> Result<(), ResourceStoreError>;
    fn new_voice_id(&self) -> Result<String, ResourceStoreError>;
    fn store_reference(
        &self,
        voice_id: &str,
        wav: &[u8],
    ) -> Result<StoredReference, ResourceStoreError>;
    fn remove_reference(&self, reference: &ReferenceAsset) -> Result<(), ResourceStoreError>;
    fn resolve_reference(&self, reference: &ReferenceAsset) -> Result<String, ResourceStoreError>;
}
