//! 有来源、有方向的公开关系事实；不把同场参与推断为朋友。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntityKind {
    Viewer,
    Topic,
    Activity,
    Unresolved,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationEntity {
    pub kind: EntityKind,
    pub id: String,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelationKind {
    Mention,
    Acquaintance,
    Participated,
    SharedInterest,
    Friend,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelationConfirmation {
    Claimed,
    Confirmed,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationEvidence {
    pub source: String,
    pub event_id: String,
    pub quote: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationshipFact {
    pub id: String,
    pub version: u64,
    pub source: RelationEntity,
    pub target: RelationEntity,
    pub kind: RelationKind,
    pub confirmation: RelationConfirmation,
    pub evidence: Vec<RelationEvidence>,
    pub expires_at_ms: Option<u64>,
    pub deleted: bool,
}
