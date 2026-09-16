mod atom;
mod pool;
mod post;
mod tag;
mod user;

pub mod scopes {
    pub use super::pool::*;
    pub use super::post::*;
    pub use super::tag::*;
    pub use super::user::*;
}

#[allow(unused_imports)] // used by tests only
pub use atom::{Comparator, PredicateParseError};

use anyhow::Result;
use smallvec::SmallVec;

use crate::{auth::RoleKey, post::CollectionKey};

pub trait ResourceContext {
    const SCOPE: PredicateScope;
    fn matches(&self, conds: &ScopedPredicates) -> bool;
}

pub trait PredicateResolver {
    fn lookup_role(&self, id: &str) -> Option<RoleKey>;
    fn lookup_collection(&self, id: &str) -> Option<CollectionKey>;
}

// {{{ boilerplate generator
macro_rules! declare_scopes {
    ($($scope:ident),+ $(,)?) => {
        paste::paste! {
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub enum PredicateScope {
                $($scope),+
            }

            impl PredicateScope {
                pub fn parse_conds(
                    &self,
                    raw: &[&str],
                    resolver: &impl PredicateResolver,
                ) -> Result<ScopedPredicates> {
                    match self {
                        $(Self::$scope => {
                            let conds: SmallVec<[scopes::[<$scope Predicate>]; 4]> =
                                raw
                                .into_iter()
                                .map(|raw| scopes::[<$scope Predicate>]::parse(
                                    raw,
                                    resolver,
                                ))
                                .collect::<Result<_>>()?;
                            Ok(ScopedPredicates::$scope(conds))
                        }),+
                    }
                }
            }

            #[derive(Clone, Debug)]
            pub enum ScopedPredicates {
                $($scope(SmallVec<[scopes::[<$scope Predicate>]; 4]>)),+
            }

            impl ScopedPredicates {
                pub fn len(&self) -> usize {
                    match self {
                        $(Self::$scope(conds) => conds.len()),+
                    }
                }

                pub fn scope(&self) -> PredicateScope {
                    match self {
                        $(Self::$scope(_) => PredicateScope::$scope),+
                    }
                }
            }

            $(
                impl ResourceContext for scopes::[<$scope Context>] {
                    const SCOPE: PredicateScope = PredicateScope::$scope;
                    fn matches(&self, conds: &ScopedPredicates) -> bool {
                        if let ScopedPredicates::$scope(p) = conds {
                             p.iter().all(|p| p.matches(self))
                        } else {
                            false
                        }
                    }
                }
            )+
        }
    };
}
// }}}

// declares PredicateScope, ScopedPredicates, and ResourceContext impls
declare_scopes! {
    Pool,
    Post,
    Tag,
    User,
}
