use moka::future::Cache;
use serde::Deserialize;

/// integer is db primary key
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId(usize);

#[derive(Clone)]
pub struct User {
    pub id: UserId,
    pub role: RoleKey,
}

/// not a primary key. role_a > role_b means that role_a is higher
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoleKey(pub u16);

impl RoleKey {
    pub fn i(&self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,

    #[serde(default)]
    pub owner: bool,
    #[serde(default)]
    pub user: bool,
    #[serde(default)]
    pub guest: bool,
}

// TODO: interface for handling user login, cache invalidation, etc
// TODO: it would probably be nice to have a check permission wrapper that takes in a
// user and a generic model object like Post or Tag and internally generates *Context
pub struct Auth {
    // cache of user auth data
    pub user_cache: Cache<UserId, User>,
}

impl Auth {
    pub fn new(user_cache_size: u64) -> Self {
        Auth {
            user_cache: Cache::new(user_cache_size),
        }
    }
}
