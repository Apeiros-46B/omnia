use std::{collections::HashMap, ops::Deref, sync::Arc};

use anyhow::Result;

use crate::{
    auth::Auth,
    config::Config,
    db::Db,
    perms::{PermissionTable, RuleDef},
    registry::Registry,
    secrets::Secrets,
};

#[derive(Clone)]
pub struct AppState(Arc<State>);

pub struct State {
    pub db: Db,
    pub auth: Auth,
    pub perms: PermissionTable,
    pub registry: Registry,
    pub secrets: Secrets,
}

impl Deref for AppState {
    type Target = Arc<State>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AppState {
    /// Create a new application state and connect to the database.
    pub async fn new(config: Config) -> Result<Self> {
        let registry = Registry::try_new(config.roles, config.collections)?;
        let perms = build_perms(&config.permissions, &registry)?;

        let db = Db::open(&config.general.database_path).await?;
        let secrets = Secrets::read_or_generate(&db).await?;
        let auth = Auth::new(64);

        Ok(AppState(Arc::new(State {
            db,
            auth,
            perms,
            registry,
            secrets,
        })))
    }
}

// TODO: this feels awkward right now. in the future registry probably should not hold
// entire CollectionConfig objects, it should hold the actual Collection domain model
fn build_perms(
    global_permissions: &HashMap<String, String>,
    registry: &Registry,
) -> Result<PermissionTable> {
    let global_definitions = global_permissions.iter().map(|(expr, role_id)| RuleDef {
        expr,
        role_id,
        collection_id: None,
    });

    let collection_definitions = registry.collections().flat_map(|(id, collection)| {
        collection
            .permissions
            .iter()
            .map(move |(expr, role_id)| RuleDef {
                expr,
                role_id,
                collection_id: Some(id),
            })
    });

    PermissionTable::compile(global_definitions.chain(collection_definitions), registry)
}
