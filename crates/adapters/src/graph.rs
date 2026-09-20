//! Neo4j 5.26 Query API公开关系投影；参数化查询与版本墓碑防止迟到复活。
use futures_util::StreamExt;
use meowlive_application::ports::{
    relationships::*,
    viewers::{ViewerStoreError, ViewerStoreFuture},
};
use reqwest::{Client, Url};
use serde_json::{Value, json};
use std::time::Duration;
#[derive(Clone)]
pub struct Neo4jRelationshipGraph {
    client: Client,
    endpoint: Url,
    username: String,
    password: String,
}
fn error() -> ViewerStoreError {
    ViewerStoreError::new("Neo4j relationship operation failed")
}
fn entity(k: EntityKind) -> &'static str {
    match k {
        EntityKind::Viewer => "viewer",
        EntityKind::Topic => "topic",
        EntityKind::Activity => "activity",
        EntityKind::Unresolved => "unresolved",
    }
}
fn kind(k: RelationKind) -> &'static str {
    match k {
        RelationKind::Mention => "mention",
        RelationKind::Acquaintance => "acquaintance",
        RelationKind::Participated => "participated",
        RelationKind::SharedInterest => "shared_interest",
        RelationKind::Friend => "friend",
    }
}
fn key(s: &str) -> Result<(), ViewerStoreError> {
    if s.trim().is_empty() || s.chars().count() > 128 || s.chars().any(char::is_control) {
        return Err(error());
    }
    Ok(())
}
impl Neo4jRelationshipGraph {
    pub async fn connect(
        base_url: &str,
        username: &str,
        password: &str,
    ) -> Result<Self, ViewerStoreError> {
        let mut endpoint = Url::parse(base_url).map_err(|_| error())?;
        if !matches!(endpoint.scheme(), "http" | "https")
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || username.is_empty()
            || password.is_empty()
        {
            return Err(error());
        }
        endpoint.set_path("/db/neo4j/query/v2");
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(3))
            .build()
            .map_err(|_| error())?;
        let graph = Self {
            client,
            endpoint,
            username: username.into(),
            password: password.into(),
        };
        for statement in [
            "CREATE CONSTRAINT meowlive_fact_key IF NOT EXISTS FOR (f:MeowFact) REQUIRE (f.scope, f.id) IS UNIQUE",
            "CREATE CONSTRAINT meowlive_entity_key IF NOT EXISTS FOR (e:MeowEntity) REQUIRE (e.scope, e.kind, e.id) IS UNIQUE",
        ] {
            graph.execute(statement, json!({})).await?;
        }
        Ok(graph)
    }
    async fn execute(&self, statement: &str, parameters: Value) -> Result<Value, ViewerStoreError> {
        let response = self
            .client
            .post(self.endpoint.clone())
            .basic_auth(&self.username, Some(&self.password))
            .header("Accept", "application/json")
            .json(&json!({"statement":statement,"parameters":parameters}))
            .send()
            .await
            .map_err(|_| error())?;
        if !response.status().is_success() || response.content_length().is_some_and(|n| n > 65536) {
            return Err(error());
        }
        let mut stream = response.bytes_stream();
        let mut body = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| error())?;
            if body.len() + chunk.len() > 65536 {
                return Err(error());
            }
            body.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&body).map_err(|_| error())?;
        if value
            .get("errors")
            .and_then(Value::as_array)
            .is_some_and(|e| !e.is_empty())
        {
            return Err(error());
        }
        if value.get("data").is_none() {
            return Err(error());
        }
        Ok(value)
    }
}
impl RelationshipGraph for Neo4jRelationshipGraph {
    fn project<'a>(&'a self, p: &'a GraphProjection) -> ViewerStoreFuture<'a, ()> {
        Box::pin(async move {
            key(&p.scope)?;
            key(&p.fact.id)?;
            key(&p.fact.source.id)?;
            key(&p.fact.target.id)?;
            if p.fact.version == 0 || p.fact.version > i64::MAX as u64 {
                return Err(error());
            }
            // The self-dependent SET acquires the node write lock before evaluating version.
            // A deleted fact remains a node tombstone but loses all traversal edges.
            let statement = "MERGE (f:MeowFact {scope:$scope,id:$id}) ON CREATE SET f.version=0, f.lock=0 SET f.lock=coalesce(f.lock,0)+1 WITH f WHERE f.version < $version SET f.version=$version,f.deleted=$deleted,f.confirmed=$confirmed,f.kind=$kind,f.expires=$expires WITH f OPTIONAL MATCH (f)-[old:MEOW_SOURCE|MEOW_TARGET]->() DELETE old WITH DISTINCT f FOREACH (_ IN CASE WHEN NOT $deleted AND $confirmed THEN [1] ELSE [] END | MERGE (a:MeowEntity {scope:$scope,kind:$source_kind,id:$source_id}) MERGE (b:MeowEntity {scope:$scope,kind:$target_kind,id:$target_id}) MERGE (f)-[:MEOW_SOURCE]->(a) MERGE (f)-[:MEOW_TARGET]->(b)) RETURN f.id AS id";
            self.execute(statement,json!({"scope":p.scope,"id":p.fact.id,"version":p.fact.version,"deleted":p.fact.deleted,"confirmed":p.fact.confirmation==RelationConfirmation::Confirmed,"kind":kind(p.fact.kind),"expires":p.fact.expires_at_ms,"source_kind":entity(p.fact.source.kind),"source_id":p.fact.source.id,"target_kind":entity(p.fact.target.kind),"target_id":p.fact.target.id})).await?;
            Ok(())
        })
    }
    fn neighbors<'a>(
        &'a self,
        scope: &'a str,
        viewer: &'a str,
        depth: u8,
        limit: u32,
    ) -> ViewerStoreFuture<'a, Vec<GraphReference>> {
        Box::pin(async move {
            key(scope)?;
            key(viewer)?;
            if !(1..=2).contains(&depth) || !(1..=100).contains(&limit) {
                return Err(error());
            }
            let statement = if depth == 1 {
                "MATCH (e:MeowEntity {scope:$scope,kind:'viewer',id:$viewer})<-[:MEOW_SOURCE|MEOW_TARGET]-(f:MeowFact) WHERE f.scope=$scope AND NOT f.deleted AND f.confirmed AND (f.expires IS NULL OR f.expires>timestamp()) RETURN DISTINCT f.id AS id,f.version AS version LIMIT $limit"
            } else {
                "MATCH (e:MeowEntity {scope:$scope,kind:'viewer',id:$viewer})<-[:MEOW_SOURCE|MEOW_TARGET]-(first:MeowFact)-[:MEOW_SOURCE|MEOW_TARGET]->(middle:MeowEntity)<-[:MEOW_SOURCE|MEOW_TARGET]-(f:MeowFact) WHERE first.scope=$scope AND middle.scope=$scope AND f.scope=$scope AND NOT first.deleted AND first.confirmed AND (first.expires IS NULL OR first.expires>timestamp()) AND NOT f.deleted AND f.confirmed AND (f.expires IS NULL OR f.expires>timestamp()) RETURN DISTINCT f.id AS id,f.version AS version LIMIT $limit"
            };
            let value = self
                .execute(
                    statement,
                    json!({"scope":scope,"viewer":viewer,"limit":limit}),
                )
                .await?;
            let fields = value
                .pointer("/data/fields")
                .and_then(Value::as_array)
                .ok_or_else(error)?;
            let id_index = fields
                .iter()
                .position(|v| v.as_str() == Some("id"))
                .ok_or_else(error)?;
            let version_index = fields
                .iter()
                .position(|v| v.as_str() == Some("version"))
                .ok_or_else(error)?;
            let rows = value
                .pointer("/data/values")
                .and_then(Value::as_array)
                .ok_or_else(error)?;
            if rows.len() > limit as usize {
                return Err(error());
            }
            rows.iter()
                .map(|r| {
                    Ok(GraphReference {
                        fact_id: r
                            .get(id_index)
                            .and_then(Value::as_str)
                            .ok_or_else(error)?
                            .into(),
                        version: r
                            .get(version_index)
                            .and_then(Value::as_u64)
                            .ok_or_else(error)?,
                    })
                })
                .collect()
        })
    }
}
