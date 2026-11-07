use anyhow::{Result, bail};

use super::{atom::{Comparator, PredicateAtom, parse_val}, PredicateResolver};
use crate::post::{CollectionKey, Rating};

pub struct PostContext {
    pub rating: Rating,
    pub score: i64,
    pub owned: bool,
    pub collection: CollectionKey,
}

#[derive(Clone, Debug)]
pub enum PostPredicate {
    Rating { cmp: Comparator, rating: Rating },
    Score { cmp: Comparator, score: i64 },
    Owned(bool), // whether or not the user owns the post
    Collection(CollectionKey),
}

impl PostPredicate {
    pub fn matches(&self, ctx: &PostContext) -> bool {
        match self {
            Self::Rating { cmp, rating } => cmp.cmp(ctx.rating, *rating),
            Self::Score { cmp, score } => cmp.cmp(ctx.score, *score),
            Self::Owned(owned) => ctx.owned == *owned,
            Self::Collection(collection) => ctx.collection == *collection,
        }
    }

    pub fn parse(src: &str, resolver: &impl PredicateResolver) -> Result<Self> {
        match PredicateAtom::parse(src)? {
            PredicateAtom::Comparison {
                key: "rating",
                cmp,
                val,
            } => Ok(Self::Rating { cmp, rating: parse_val(val)? }),

            PredicateAtom::Comparison {
                key: "score",
                cmp,
                val,
            } => Ok(Self::Score { cmp, score: parse_val(val)? }),

            PredicateAtom::Comparison {
                key: "collection",
                cmp: Comparator::Eq,
                val,
            } => {
                let Some(col) = resolver.lookup_collection(val) else {
                    bail!("unknown collection '{val}'")
                };
                Ok(Self::Collection(col))
            },

            PredicateAtom::Flag { key: "owned", val } => Ok(Self::Owned(val)),

            _ => bail!("invalid post predicate"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        post::{CollectionKey, Rating},
        predicate::Comparator,
    };

    use super::{PostContext, PostPredicate};

    fn post_ctx(rating: Rating, score: i64, owned: bool, col: u32) -> PostContext {
        PostContext {
            rating,
            score,
            owned,
            collection: CollectionKey(col),
        }
    }

    #[test]
    fn match_rating() {
        let cond = PostPredicate::Rating {
            cmp: Comparator::Lte,
            rating: Rating::Safe,
        };
        let ctx_safe = post_ctx(Rating::Safe, 0, false, 0);
        let ctx_risky = post_ctx(Rating::Risky, 0, false, 0);
        let ctx_unsafe = post_ctx(Rating::Unsafe, 0, false, 0);

        assert!(cond.matches(&ctx_safe));
        assert!(!cond.matches(&ctx_risky));
        assert!(!cond.matches(&ctx_unsafe));
    }

    #[test]
    fn match_score() {
        let cond_score_gt = PostPredicate::Score {
            cmp: Comparator::Gt,
            score: 50,
        };
        let cond_score_lt = PostPredicate::Score {
            cmp: Comparator::Lt,
            score: 50,
        };
        let ctx = post_ctx(Rating::Safe, 100, false, 0);

        assert!(cond_score_gt.matches(&ctx));
        assert!(!cond_score_lt.matches(&ctx));
    }

    #[test]
    fn match_owned() {
        let cond_owned = PostPredicate::Owned(true);
        let cond_other = PostPredicate::Owned(false);
        let ctx_owned = post_ctx(Rating::Safe, 0, true, 0);
        let ctx_other = post_ctx(Rating::Safe, 0, false, 0);

        assert!(cond_owned.matches(&ctx_owned));
        assert!(!cond_owned.matches(&ctx_other));
        assert!(cond_other.matches(&ctx_other));
        assert!(!cond_other.matches(&ctx_owned));
    }

    #[test]
    fn match_collection() {
        let cond_collection0 = PostPredicate::Collection(CollectionKey(0));
        let cond_collection1 = PostPredicate::Collection(CollectionKey(1));
        let ctx = post_ctx(Rating::Safe, 0, false, 0);

        assert!(cond_collection0.matches(&ctx));
        assert!(!cond_collection1.matches(&ctx));
    }
}
