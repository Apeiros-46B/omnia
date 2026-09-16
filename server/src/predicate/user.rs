use anyhow::{Result, bail};

use crate::auth::RoleKey;

use super::atom::{Comparator, PredicateAtom, parse_val};

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

    pub fn parse(src: &str) -> Result<Self> {
        match PredicateAtom::parse(src)? {
            // TODO: need to get role key from id to key mapping and then add tests
            // PredicateAtom::Comparison {
            //     key: "role",
            //     cmp,
            //     val,
            // } => Ok(Self::Role { cmp, role: parse_val(val)? }),

            _ => bail!("invalid user predicate"),
        }
    }
}

#[cfg(test)]
mod tests {
}
