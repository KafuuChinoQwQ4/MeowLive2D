//! 持久化和本地素材存储实现；应用层只看存取接口。

pub mod files;
pub mod postgres;
pub mod resources;
pub mod sqlite;

pub mod receipt_journal;
