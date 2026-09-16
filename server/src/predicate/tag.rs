use anyhow::{Result, bail};

use super::atom::{Comparator, PredicateAtom, parse_val};

pub struct TagContext {
    pub posts: u64,
}

#[derive(Clone, Debug)]
pub enum TagPredicate {
    Posts { cmp: Comparator, posts: u64 },
}

impl TagPredicate {
    pub fn matches(&self, ctx: &TagContext) -> bool {
        match self {
            Self::Posts { cmp, posts } => cmp.cmp(ctx.posts, *posts),
        }
    }

    pub fn parse(src: &str) -> Result<Self> {
        match PredicateAtom::parse(src)? {
            PredicateAtom::Comparison {
                key: "posts",
                cmp,
                val,
            } => Ok(Self::Posts { cmp, posts: parse_val(val)? }),

            _ => bail!("invalid tag predicate"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{predicate::Comparator};

    use super::{TagContext, TagPredicate};

    fn tag_ctx(posts: u64) -> TagContext {
        TagContext { posts }
    }

    #[test]
    fn match_posts() {
        let cond_posts_gt = TagPredicate::Posts {
            cmp: Comparator::Gt,
            posts: 50,
        };
        let cond_posts_lt = TagPredicate::Posts {
            cmp: Comparator::Lt,
            posts: 50,
        };
        let ctx = tag_ctx(100);

        assert!(cond_posts_gt.matches(&ctx));
        assert!(!cond_posts_lt.matches(&ctx));
    }
}
