mod keys;
mod rule;

use keys::Permission;
use rule::Rule;

use anyhow::{Result, bail};

use crate::{
    auth::RoleKey,
    predicate::{
        PredicateResolver, ResourceContext, ScopedPredicates, scopes::PostPredicate,
    },
};

pub struct RuleDef<'a> {
    pub expr: &'a str,
    pub role_id: &'a str,
    pub collection_id: Option<&'a str>,
}

#[derive(Debug)]
pub struct PermissionTable([Box<[Rule]>; Permission::COUNT]);

impl PermissionTable {
    // {{{ build from definitions
    pub fn compile<'a>(
        defs: impl IntoIterator<Item = RuleDef<'a>>,
        resolver: &impl PredicateResolver,
    ) -> Result<Self> {
        let mut tbl_pre: [Vec<Rule>; Permission::COUNT] = Default::default();
        let perm_expr_re = regex::regex!(r"^([a-z.]+(?:\.\*)?|\*)(\([^)]+\))?$");

        for def in defs {
            let Some(caps) = perm_expr_re.captures(def.expr) else {
                bail!("permission expression '{}' is malformed", def.expr);
            };

            let Some(role) = resolver.lookup_role(def.role_id) else {
                bail!("unknown role '{}' for '{}'", def.role_id, def.expr);
            };
            let collection = if let Some(id) = def.collection_id {
                let Some(collection) = resolver.lookup_collection(id) else {
                    bail!("unknown collection '{}' for '{}'", id, def.expr);
                };
                Some(collection)
            } else {
                None
            };

            let key = caps.get(1).unwrap().as_str();
            if key == "*" {
                if caps.get(2).is_some() {
                    bail!("predicates cannot be used with '*'");
                }
                if def.collection_id.is_some() {
                    bail!("collection-specific permissions cannot be '*'");
                }
            }

            let perms = Permission::from_key(key)?;
            let exact = !key.ends_with('*');

            let raw_conds: Vec<&str> = caps.get(2)
                .map(|capture| {
                    let source = capture.as_str();
                    source[1..source.len() - 1].split(',').collect()
                })
                .unwrap_or_default();

            let mut rules = Vec::with_capacity(perms.len());
            for perm in perms {
                let mut conds = perm.scope().parse_conds(&raw_conds, resolver)?;
                if let Some(collection) = collection {
                    let ScopedPredicates::Post(post_conds) = &mut conds else {
                        bail!(
                            "collection-specific perm '{}' must be of post scope",
                            perm,
                        );
                    };
                    post_conds.push(PostPredicate::Collection(collection));
                }
                rules.push(Rule::new(perm, role, conds, exact)?);
            }

            for rule in rules {
                tbl_pre[rule.perm.as_usize()].push(rule);
            }
        }

        let mut tbl: [Box<[Rule]>; Permission::COUNT] = Default::default();

        for (i, mut rules) in tbl_pre.into_iter().enumerate() {
            rules.sort_unstable_by(|a, b| b.cmp_prio(a));
            tbl[i] = rules.into_boxed_slice();
        }

        Ok(PermissionTable(tbl))
    }
    // }}}

    // in debug mode, scope check has an assert. in release, fail-closed
    #[allow(private_bounds)]
    pub fn check<C: ResourceContext>(
        &self,
        permission: Permission,
        context: &C,
        role: RoleKey,
    ) -> bool {
        if permission.scope() != C::SCOPE {
            debug_assert_eq!(permission.scope(), C::SCOPE);
            return false;
        }

        // rules are pre-sorted by priority, strongest first
        for rule in &self.0[permission.as_usize()] {
            if context.matches(rule.conds()) {
                return role >= rule.role;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        auth::RoleKey,
        post::{CollectionKey, Rating},
        predicate::{
            PredicateResolver,
            scopes::PostContext,
        },
    };

    use super::{Permission, PermissionTable, RuleDef};

    struct TestResolver;

    impl PredicateResolver for TestResolver {
        fn lookup_role(&self, id: &str) -> Option<RoleKey> {
            match id {
                "role0" => Some(RoleKey(0)),
                "role1" => Some(RoleKey(1)),
                "role2" => Some(RoleKey(2)),
                "role3" => Some(RoleKey(3)),
                _ => None,
            }
        }

        fn lookup_collection(&self, id: &str) -> Option<CollectionKey> {
            match id {
                "collection0" => Some(CollectionKey(0)),
                "collection1" => Some(CollectionKey(1)),
                _ => None,
            }
        }
    }

    fn post_context(rating: Rating, owned: bool) -> PostContext {
        PostContext {
            rating,
            score: 0,
            owned,
            collection: CollectionKey(0),
        }
    }

    #[test]
    fn permission_construction_succeeds() {
        let defs = [
            RuleDef { expr: "post.view", role_id: "role0", collection_id: None },
            RuleDef { expr: "post.edit.tags", role_id: "role1", collection_id: None },
            RuleDef {
                expr: "post.edit.tags(owned)",
                role_id: "role2",
                collection_id: None,
            },
        ];

        assert!(PermissionTable::compile(defs, &TestResolver).is_ok());
    }

    #[test]
    fn permission_construction_rejects_mismatched_scope() {
        let defs = [
            RuleDef {
                expr: "post.edit.tags(posts>50)",
                role_id: "role3",
                collection_id: None,
            },
            RuleDef {
                expr: "pool.delete(rating!=safe)",
                role_id: "role3",
                collection_id: None,
            },
        ];

        assert!(PermissionTable::compile(defs, &TestResolver).is_err());
    }

    #[test]
    fn permission_construction_rejects_predicates_on_root_wildcard() {
        let defs = [RuleDef {
            expr: "*(owned)",
            role_id: "role0",
            collection_id: None,
        }];

        assert!(PermissionTable::compile(defs, &TestResolver).is_err());
    }

    #[test]
    fn permission_construction_rejects_unknown_role() {
        let defs = [RuleDef {
            expr: "post.view",
            role_id: "unknown",
            collection_id: None,
        }];

        assert!(PermissionTable::compile(defs, &TestResolver).is_err());
    }

    #[test]
    fn permission_construction_rejects_unknown_collection() {
        let defs = [RuleDef {
            expr: "post.view",
            role_id: "role0",
            collection_id: Some("unknown"),
        }];

        assert!(PermissionTable::compile(defs, &TestResolver).is_err());
    }

    #[test]
    fn collection_permission_only_matches_its_collection() {
        let permissions = PermissionTable::compile(
            [RuleDef {
                expr: "post.view",
                role_id: "role0",
                collection_id: Some("collection1"),
            }],
            &TestResolver,
        )
        .unwrap();

        let mut context = post_context(Rating::Safe, false);
        assert!(!permissions.check(Permission::PostView, &context, RoleKey(0)));

        context.collection = CollectionKey(1);
        assert!(permissions.check(Permission::PostView, &context, RoleKey(0)));
    }

    #[test]
    fn collection_permission_overrides_more_specific_global_rule() {
        let permissions = PermissionTable::compile(
            [
                RuleDef {
                    expr: "post.view(owned,rating=safe)",
                    role_id: "role3",
                    collection_id: None,
                },
                RuleDef {
                    expr: "post.view",
                    role_id: "role0",
                    collection_id: Some("collection1"),
                },
            ],
            &TestResolver,
        )
        .unwrap();

        let mut context = post_context(Rating::Safe, true);
        context.collection = CollectionKey(1);
        assert!(permissions.check(Permission::PostView, &context, RoleKey(0)));

        context.collection = CollectionKey(0);
        assert!(!permissions.check(Permission::PostView, &context, RoleKey(0)));
    }

    #[test]
    fn collection_permission_rejects_non_post_scope() {
        let defs = [RuleDef {
            expr: "pool.view",
            role_id: "role0",
            collection_id: Some("collection1"),
        }];

        assert!(PermissionTable::compile(defs, &TestResolver).is_err());
    }

    #[test]
    fn permission_check_implicitly_denies() {
        let permissions = PermissionTable::compile(
            std::iter::empty::<RuleDef<'static>>(),
            &TestResolver,
        )
        .unwrap();
        assert!(!permissions.check(
            Permission::PostView,
            &post_context(Rating::Safe, false),
            RoleKey(0),
        ));
    }

    #[test]
    fn permission_check_uses_rule_priority() {
        let permissions = PermissionTable::compile(
            [
                // A: weakest
                RuleDef {
                    expr: "*",
                    role_id: "role0",
                    collection_id: None,
                },
                // B: strongest
                RuleDef {
                    expr: "post.view(owned,rating=safe)",
                    role_id: "role1",
                    collection_id: None,
                },
                // C: stronger than E, weaker than D
                RuleDef {
                    expr: "post.*(owned)",
                    role_id: "role1",
                    collection_id: None,
                },
                // D: stronger than C, weaker than B
                RuleDef {
                    expr: "post.view(owned)",
                    role_id: "role0",
                    collection_id: None,
                },
                // E: stronger than A
                RuleDef {
                    expr: "post.*",
                    role_id: "role1",
                    collection_id: None,
                },
            ],
            &TestResolver,
        )
        .unwrap();

        assert!(!permissions.check(
            Permission::PostView,
            &post_context(Rating::Safe, true),
            RoleKey(0),
        ));
        assert!(permissions.check(
            Permission::PostView,
            &post_context(Rating::Risky, true),
            RoleKey(0),
        ));
        assert!(!permissions.check(
            Permission::PostView,
            &post_context(Rating::Safe, false),
            RoleKey(0),
        ));
    }
}
