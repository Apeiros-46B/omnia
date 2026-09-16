use anyhow::{Result, bail};

use super::atom::{Comparator, PredicateAtom, parse_val};

pub struct PoolContext {
    pub posts: u64,
    pub owned: bool,
}

#[derive(Clone, Debug)]
pub enum PoolPredicate {
    Posts { cmp: Comparator, posts: u64 },
    Owned(bool), // whether or not the user owns the pool
}

impl PoolPredicate {
    pub fn matches(&self, ctx: &PoolContext) -> bool {
        match self {
            Self::Posts { cmp, posts } => cmp.cmp(ctx.posts, *posts),
            Self::Owned(owned) => ctx.owned == *owned,
        }
    }

    pub fn parse(src: &str) -> Result<Self> {
        match PredicateAtom::parse(src)? {
            PredicateAtom::Comparison {
                key: "posts",
                cmp,
                val,
            } => Ok(Self::Posts { cmp, posts: parse_val(val)? }),

            PredicateAtom::Flag { key: "owned", val } => Ok(Self::Owned(val)),

            _ => bail!("invalid pool predicate"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{predicate::Comparator};

    use super::{PoolContext, PoolPredicate};

    fn pool_ctx(posts: u64, owned: bool) -> PoolContext {
        PoolContext { posts, owned }
    }

    #[test]
    fn match_posts() {
        let cond_posts_gt = PoolPredicate::Posts {
            cmp: Comparator::Gt,
            posts: 50,
        };
        let cond_posts_lt = PoolPredicate::Posts {
            cmp: Comparator::Lt,
            posts: 50,
        };
        let ctx = pool_ctx(100, false);

        assert!(cond_posts_gt.matches(&ctx));
        assert!(!cond_posts_lt.matches(&ctx));
    }

    #[test]
    fn match_owned() {
        let cond_owned = PoolPredicate::Owned(true);
        let cond_other = PoolPredicate::Owned(false);
        let ctx_owned = PoolContext { posts: 0, owned: true };
        let ctx_other = PoolContext { posts: 0, owned: false };

        assert!(cond_owned.matches(&ctx_owned));
        assert!(!cond_owned.matches(&ctx_other));
        assert!(cond_other.matches(&ctx_other));
        assert!(!cond_other.matches(&ctx_owned));
    }
}
