mod keys;
mod rule;

use anyhow::{Result, bail};
use crate::{
    auth::RoleKey,
    post::Collection,
    predicate::{
        scopes::PostPredicate,
        ResourceContext,
        ScopedPredicates,
    },
};

pub use keys::Permission;
use rule::Rule;

#[derive(Debug)]
pub struct PermissionTable([Box<[Rule]>; Permission::COUNT]);

impl PermissionTable {
    // in debug mode, scope check has an assert. fail-closed in release
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

#[derive(Debug, Default)]
pub struct PermissionTableBuilder([Vec<Rule>; Permission::COUNT]);

impl PermissionTableBuilder {
    fn register(&mut self, rule: Rule) {
        self.0[rule.perm.as_usize()].push(rule);
    }

    // TODO: cleanup
    pub fn parse_and_register<K: AsRef<str>>(
        &mut self,
        definitions: impl Iterator<Item = (K, RoleKey)>,
        collection: Option<Collection>,
    ) -> Result<()> {
        let perm_lhs = regex::regex!(r"^([a-z.]+(?:\.\*)?|\*)(\([^)]+\))?$");

        for (lhs, role) in definitions {
            let Some(caps) = perm_lhs.captures(lhs.as_ref()) else {
                bail!("permission key '{}' is malformed", lhs.as_ref());
            };

            let key = caps.get(1).unwrap().as_str();
            if key == "*" {
                if caps.get(2).is_some() {
                    bail!("predicates cannot be used with '*'");
                }
                if collection.is_some() {
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
                let mut conds = perm.scope().parse_conds(&raw_conds)?;
                if let Some(collection) = collection {
                    let ScopedPredicates::Post(post_conds) = &mut conds else {
                        bail!(
                            "collection-specific permission '{}' must have post scope",
                            perm,
                        );
                    };
                    post_conds.push(PostPredicate::Collection(collection));
                }
                rules.push(Rule::new(perm, role, conds, exact)?);
            }

            for rule in rules {
                self.register(rule);
            }
        }

        Ok(())
    }

    pub fn build(self) -> PermissionTable {
        let mut permissions: [Box<[Rule]>; Permission::COUNT] = Default::default();

        for (index, mut rules) in self.0.into_iter().enumerate() {
            rules.sort_unstable_by(|a, b| b.cmp_prio(a));
            permissions[index] = rules.into_boxed_slice();
        }

        PermissionTable(permissions)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        auth::RoleKey,
        post::{Collection, Rating},
        predicate::{
            scopes::{PostContext, PostPredicate},
            Comparator,
            ScopedPredicates,
        },
    };

    use super::{Permission, PermissionTableBuilder, Rule};

    fn post_context(rating: Rating, owned: bool) -> PostContext {
        PostContext {
            rating,
            score: 0,
            owned,
            collection: Collection(0),
        }
    }

    fn post_rule(
        permission: Permission,
        conds: impl IntoIterator<Item = PostPredicate>,
        role: u16,
        exact: bool,
    ) -> Rule {
        Rule::new(
            permission,
            RoleKey(role),
            ScopedPredicates::Post(conds.into_iter().collect()),
            exact,
        )
        .unwrap()
    }

    #[test]
    fn permission_construction_succeeds() {
        let mut raw = HashMap::new();
        raw.insert("post.view", 0);
        raw.insert("post.edit.tags", 1);
        raw.insert("post.edit.tags(owned)", 2);

        let permissions = raw
            .iter()
            .map(|(key, role)| (key.to_string(), RoleKey(*role)));
        let mut builder = PermissionTableBuilder::default();
        assert!(builder.parse_and_register(permissions, None).is_ok());
    }

    #[test]
    fn permission_construction_rejects_mismatched_scope() {
        let mut raw = HashMap::new();
        raw.insert("post.edit.tag(posts>50)", 3);
        raw.insert("pool.delete(rating!=safe)", 3);

        let permissions = raw
            .iter()
            .map(|(key, role)| (key.to_string(), RoleKey(*role)));
        let mut builder = PermissionTableBuilder::default();
        assert!(builder.parse_and_register(permissions, None).is_err());
    }

    #[test]
    fn permission_construction_rejects_predicates_on_root_wildcard() {
        let permissions = [("*(owned)", RoleKey(0))].into_iter();
        let mut builder = PermissionTableBuilder::default();

        assert!(builder.parse_and_register(permissions, None).is_err());
    }

    #[test]
    fn collection_permission_only_matches_its_collection() {
        let permissions = [("post.view", RoleKey(0))].into_iter();
        let mut builder = PermissionTableBuilder::default();
        builder
            .parse_and_register(permissions, Some(Collection(1)))
            .unwrap();
        let permissions = builder.build();

        let mut context = post_context(Rating::Safe, false);
        assert!(!permissions.check(Permission::PostView, &context, RoleKey(0)));

        context.collection = Collection(1);
        assert!(permissions.check(Permission::PostView, &context, RoleKey(0)));
    }

    #[test]
    fn collection_permission_rejects_non_post_scope() {
        let permissions = [("pool.view", RoleKey(0))].into_iter();
        let mut builder = PermissionTableBuilder::default();

        assert!(
            builder
                .parse_and_register(permissions, Some(Collection(1)))
                .is_err()
        );
    }

    #[test]
    fn permission_check_implicitly_denies() {
        let permissions = PermissionTableBuilder::default().build();
        assert!(!permissions.check(
            Permission::PostView,
            &post_context(Rating::Safe, false),
            RoleKey(0),
        ));
    }

    #[test]
    fn permission_check_uses_rule_priority() {
        let mut builder = PermissionTableBuilder::default();
        let owned = PostPredicate::Owned(true);
        let safe = PostPredicate::Rating {
            cmp: Comparator::Eq,
            rating: Rating::Safe,
        };

        // A: weakest
        builder.register(post_rule(Permission::PostView, [], 0, true));
        // B: strongest
        builder.register(post_rule(
            Permission::PostView,
            [owned.clone(), safe],
            1,
            true,
        ));
        // C: stronger than E, weaker than D
        builder.register(post_rule(Permission::PostView, [owned.clone()], 1, false));
        // D: stronger than C, weaker than B
        builder.register(post_rule(Permission::PostView, [owned], 0, true));
        // E: stronger than A
        builder.register(post_rule(Permission::PostView, [], 1, true));

        let permissions = builder.build();

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
