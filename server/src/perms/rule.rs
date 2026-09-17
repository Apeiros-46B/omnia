use std::cmp::Ordering;

use anyhow::{Result, bail};

use crate::{
    auth::RoleKey,
    predicate::{ScopedPredicates, scopes::PostPredicate},
};

use super::Permission;

#[derive(Clone, Debug)]
pub(super) struct Rule {
    pub(super) perm: Permission,
    pub(super) role: RoleKey,
    conds: ScopedPredicates,
    exact: bool,
}

impl Rule {
    pub(super) fn new(
        perm: Permission,
        role: RoleKey,
        conds: ScopedPredicates,
        exact: bool,
    ) -> Result<Self> {
        if perm.scope() != conds.scope() {
            bail!(
                "permission '{}' cannot use {:?} predicates",
                perm,
                conds.scope(),
            );
        }

        Ok(Self {
            perm,
            role,
            exact,
            conds,
        })
    }

    pub(super) fn conds(&self) -> &ScopedPredicates {
        &self.conds
    }

    // TODO: probably order by wildcard depth instead of just wildcard or not
    fn prio(&self) -> (bool, usize, bool, usize, RoleKey) {
        let col_specific = if let ScopedPredicates::Post(conds) = &self.conds {
            conds.iter().any(|p| matches!(p, PostPredicate::Collection(_)))
        } else {
            false
        };

        (
            col_specific,
            self.conds.len(),
            self.exact,
            self.perm.path_depth(),
            self.role,
        )
    }

    pub(super) fn cmp_prio(&self, other: &Self) -> Ordering {
        self.prio().cmp(&other.prio())
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use smallvec::smallvec;

    use crate::{
        auth::RoleKey,
        post::{CollectionKey, Rating},
        predicate::{Comparator, ScopedPredicates, scopes::PostPredicate},
    };

    use super::{Permission, Rule};

    fn rule(
        permission: Permission,
        predicates: impl IntoIterator<Item = PostPredicate>,
        role: u16,
        exact: bool,
    ) -> Rule {
        Rule::new(
            permission,
            RoleKey(role),
            ScopedPredicates::Post(predicates.into_iter().collect()),
            exact,
        )
        .unwrap()
    }

    #[test]
    fn construction_rejects_scope_mismatch() {
        let res = Rule::new(
            Permission::UserInvite,
            RoleKey(0),
            ScopedPredicates::Post(smallvec![]),
            true,
        );
        assert!(res.is_err());
    }

    #[test]
    fn higher_role_has_higher_priority() {
        let a = rule(Permission::PostView, [], 1, true);
        let b = rule(Permission::PostView, [], 0, true);

        assert_eq!(a.cmp_prio(&b), Ordering::Greater);
    }

    #[test]
    fn deeper_permission_has_higher_priority() {
        let a = rule(Permission::PostView, [], 1, true);
        let b = rule(Permission::PostEditTags, [], 0, true);

        assert_eq!(a.cmp_prio(&b), Ordering::Less);
    }

    #[test]
    fn exact_rule_has_higher_priority_than_wildcard() {
        let a = rule(Permission::PostEditTags, [], 1, false);
        let b = rule(Permission::PostView, [], 0, true);

        assert_eq!(a.cmp_prio(&b), Ordering::Less);
    }

    #[test]
    fn more_predicates_have_higher_priority() {
        let owned = PostPredicate::Owned(true);
        let rating = PostPredicate::Rating {
            cmp: Comparator::Lte,
            rating: Rating::Safe,
        };
        let a = rule(Permission::PostEditTags, [owned.clone()], 1, true);
        let b = rule(Permission::PostView, [owned, rating], 0, false);

        assert_eq!(a.cmp_prio(&b), Ordering::Less);
    }

    #[test]
    fn collection_specific_rule_has_highest_priority() {
        let collection_specific = rule(
            Permission::PostView,
            [PostPredicate::Collection(CollectionKey(0))],
            0,
            false,
        );
        let global = rule(
            Permission::PostEditTags,
            [
                PostPredicate::Owned(true),
                PostPredicate::Rating {
                    cmp: Comparator::Eq,
                    rating: Rating::Safe,
                },
            ],
            u16::MAX,
            true,
        );

        assert_eq!(collection_specific.cmp_prio(&global), Ordering::Greater);
    }
}
