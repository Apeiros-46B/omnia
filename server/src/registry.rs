use std::collections::HashMap;

use anyhow::{Context, Result, bail};

use crate::{
    auth::{Role, RoleKey},
    config::CollectionConfig,
    post::CollectionKey,
    predicate::PredicateResolver,
};

pub struct Registry {
    roles: RoleRegistry,
    collections: CollectionRegistry,
}

impl Registry {
    /// roles must be sorted by power, the highest role first and the lowest last.
    /// collections must contain exactly one default collection
    pub fn try_new(roles: Vec<Role>, collections: HashMap<String, CollectionConfig>)
        -> Result<Self>
    {
        Ok(Self {
            roles: RoleRegistry::new(roles)?,
            collections: CollectionRegistry::new(collections)?,
        })
    }

    pub(super) fn collections(&self)
        -> impl Iterator<Item = (&str, &CollectionConfig)>
    {
        self.collections
            .by_key
            .iter()
            .map(|entry| (entry.id.as_str(), &entry.config))
    }
}

impl PredicateResolver for Registry {
    fn lookup_role(&self, id: &str) -> Option<RoleKey> {
        self.roles.get_key(id)
    }

    fn lookup_collection(&self, id: &str) -> Option<CollectionKey> {
        self.collections.get_key(id)
    }
}

struct RoleRegistry {
    // indexed by RoleKey, first role is lowest
    pub by_key: Vec<Role>,
    pub by_id: HashMap<String, RoleKey>,
    pub owner: Option<RoleKey>,
    pub user: Option<RoleKey>,
    pub guest: Option<RoleKey>,
}

impl RoleRegistry {
    // see roles invariant of Registry
    pub fn new(mut roles: Vec<Role>) -> Result<Self> {
        roles.reverse();

        let mut by_id = HashMap::new();
        let mut owner = None;
        let mut user = None;
        let mut guest = None;

        for (i, role) in roles.iter().enumerate() {
            let key = RoleKey(u16::try_from(i).context("too many roles")?);
            // TODO: validation here or at config load time? seems weird
            if role.owner {
                owner = Some(key);
            }
            if role.user {
                user = Some(key);
            }
            if role.guest {
                guest = Some(key);
            }
            if by_id.insert(role.id.clone(), key).is_some() {
                bail!("attempted to register multiple roles with same id");
            }
        }

        Ok(Self { by_key: roles, by_id, owner, user, guest })
    }

    pub fn get(&self, key: RoleKey) -> Option<&Role> {
        self.by_key.get(key.0 as usize)
    }

    pub fn get_key(&self, id: &str) -> Option<RoleKey> {
        self.by_id.get(id).copied()
    }

    pub fn get_by_id(&self, id: &str) -> Option<&Role> {
        let key = self.get_key(id)?;
        self.get(key)
    }
}

pub struct CollectionEntry {
    id: String,
    config: CollectionConfig,
}

struct CollectionRegistry {
    // indexed by CollectionKey
    pub by_key: Vec<CollectionEntry>,
    pub by_id: HashMap<String, CollectionKey>,
    pub default: CollectionKey,
}

impl CollectionRegistry {
    // see collections invariant of Registry
    pub fn new(collections: HashMap<String, CollectionConfig>) -> Result<Self> {
        let mut by_key = Vec::new();
        let mut by_id = HashMap::new();
        let mut default = None;

        for (i, (id, collection)) in collections.into_iter().enumerate() {
            let key = CollectionKey(u32::try_from(i).context("too many collections")?);
            if collection.default {
                default = Some(key);
            }
            by_key.push(CollectionEntry { id: id.clone(), config: collection });
            by_id.insert(id, key);
        }

        let Some(default) = default else {
            bail!("no default collection configured");
        };

        Ok(Self { by_key, by_id, default })
    }

    pub fn get(&self, key: CollectionKey) -> Option<&CollectionEntry> {
        self.by_key.get(key.0 as usize)
    }

    pub fn get_key(&self, id: &str) -> Option<CollectionKey> {
        self.by_id.get(id).copied()
    }

    pub fn get_by_id(&self, id: &str) -> Option<&CollectionEntry> {
        let key = self.get_key(id)?;
        self.get(key)
    }
}
