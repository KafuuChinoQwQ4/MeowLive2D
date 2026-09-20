//! 设备已完成回执的本地持久暂存；在确认完成前写入，数据库幂等提交后删除。
use super::viewers::ViewerStoreError;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiptRecord {
    pub speech_id: String,
    pub completed_at_ms: u64,
    pub attempts: u32,
    pub last_error: Option<String>,
}
pub trait ReceiptJournal: Send + Sync {
    fn put(
        &self,
        scope: &str,
        speech_id: &str,
        completed_at_ms: u64,
    ) -> Result<(), ViewerStoreError>;
    fn pending(&self, scope: &str, limit: usize) -> Result<Vec<ReceiptRecord>, ViewerStoreError>;
    fn acknowledge(&self, scope: &str, speech_id: &str) -> Result<(), ViewerStoreError>;
    fn failed(&self, scope: &str, speech_id: &str) -> Result<(), ViewerStoreError>;
}
