use anyhow::{Result, bail};

use crate::auth::RoleKey;

use super::{atom::{Comparator, PredicateAtom}, PredicateResolver};

pub struct UserContext {
    pub role: RoleKey,
}

#[derive(Clone, Debug)]
pub enum UserPredicate {
    Role { cmp: Comparator, role: RoleKey },
}

impl UserPredicate {
    pub fn matches(&self, ctx: &UserContext) -> bool {
        match self {
            Self::Role { cmp, role } => cmp.cmp(ctx.role, *role),
        }
    }

    pub fn parse(src: &str, resolver: &impl PredicateResolver) -> Result<Self> {
        match PredicateAtom::parse(src)? {
            PredicateAtom::Comparison { key: "role", cmp, val, } => {
                let Some(role) = resolver.lookup_role(val) else {
                    bail!("unknown role '{val}'");
                };
                Ok(Self::Role { cmp, role: role })
            },

            _ => bail!("invalid user predicate"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        auth::{RoleKey},
        predicate::Comparator,
    };

    use super::{UserContext, UserPredicate};

    fn user_ctx(role: u16) -> UserContext {
        UserContext { role: RoleKey(role) }
    }

    #[test]
    fn match_score() {
        let cond_role_gt = UserPredicate::Role {
            cmp: Comparator::Gt,
            role: RoleKey(5),
        };
        let cond_role_lt = UserPredicate::Role {
            cmp: Comparator::Lt,
            role: RoleKey(5),
        };
        let ctx = user_ctx(10);

        assert!(cond_role_gt.matches(&ctx));
        assert!(!cond_role_lt.matches(&ctx));
    }
}
