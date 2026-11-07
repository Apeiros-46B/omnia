// {{{ abomination
macro_rules! define_permissions {
    ($($scope:ident $(. $rest:ident)*),* $(,)?) => { paste::paste! {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        #[repr(u8)]
        pub enum Permission {
            $(
                [<$scope $($rest)*>],
            )*
        }

        impl Permission {
            pub const COUNT: usize = [ $(Self::[<$scope $($rest)*>]),* ].len();
            pub const MEMBERS: [Self; Self::COUNT] = [
                $(Self::[<$scope $($rest)*>]),*
            ];

            const fn count_dots(s: &str) -> usize {
                let bytes = s.as_bytes();
                let mut count = 0;
                let mut i = 0;
                while i < bytes.len() {
                    if bytes[i] == b'.' {
                        count += 1;
                    }
                    i += 1;
                }
                count
            }

            /// return the number of dots in the member's string key
            pub const fn path_depth(self) -> usize {
                match self {
                    $(
                        Self::[<$scope $($rest)*>] => {
                            const DEPTH: usize = Permission::count_dots(
                                concat!(
                                    stringify!([<$scope:lower>])
                                    $(, ".", stringify!([<$rest:lower>]))*
                                )
                            );
                            DEPTH
                        },
                    )*
                }
            }

            pub const fn scope(&self) -> $crate::predicate::PredicateScope {
                match self {
                    $(
                        Self::[<$scope $($rest)*>] => $crate::predicate::PredicateScope::$scope,
                    )*
                }
            }

            /// no string validation done
            pub fn from_key(value: &str) -> anyhow::Result<Vec<Self>> {
                if let Some(prefix) = value.strip_suffix('*') {
                    let mut matches = Vec::new();

                    $(
                        if concat!(stringify!([<$scope:lower>]) $(, ".", stringify!([<$rest:lower>]))*).starts_with(prefix) {
                            matches.push(Self::[<$scope $($rest)*>]);
                        }
                    )*

                    if matches.is_empty() {
                        anyhow::bail!("no permissions matched '{}'", value);
                    }
                    Ok(matches)
                } else {
                    match value {
                        $(
                            concat!(stringify!([<$scope:lower>]) $(, ".", stringify!([<$rest:lower>]))*) => Ok(vec![Self::[<$scope $($rest)*>]]),
                        )*
                        _ => anyhow::bail!("unknown permission key '{}'", value),
                    }
                }
            }

            pub fn as_usize(self) -> usize {
                self as usize
            }
        }

        impl std::fmt::Display for Permission {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>)
                -> Result<(), std::fmt::Error>
            {
                match self {
                    $(
                        Self::[<$scope $($rest)*>] => write!(f, concat!(stringify!([<$scope:lower>]) $(, ".", stringify!([<$rest:lower>]))*)),
                    )*
                }
            }
        }
    } };
}
// }}}

define_permissions! {
    Pool.View,
    Pool.Create,
    Pool.Delete,
    Pool.Edit.Name,
    Pool.Edit.Posts,

    Post.View,
    Post.Create,
    Post.Delete,
    Post.Edit.Tags,
    Post.Edit.Meta,
    Post.Edit.Notes,
    Post.Move.Out,
    Post.Move.In,

    Tag.Delete,
    Tag.Edit.Name,
    Tag.Implication.Create,
    Tag.Implication.Edit,
    Tag.Implication.Delete,
    Tag.Alias.Create,
    Tag.Alias.Edit,
    Tag.Alias.Delete,

    User.Invite,
    User.Edit.Role,
    User.Mod.Mute,
    User.Mod.Ban,
}

#[cfg(test)]
mod tests {
    use crate::predicate::PredicateScope;

    use super::Permission;

    #[test]
    fn parses_exact_permission() {
        let result = Permission::from_key("post.view").unwrap();
        assert_eq!(result, vec![Permission::PostView]);
    }

    #[test]
    fn expands_scoped_wildcard() {
        let mut result = Permission::from_key("post.move.*").unwrap();
        let mut expected = vec![Permission::PostMoveIn, Permission::PostMoveOut];
        result.sort_unstable();
        expected.sort_unstable();
        assert_eq!(result, expected);
    }

    #[test]
    fn expands_global_wildcard() {
        let result = Permission::from_key("*").unwrap();
        assert_eq!(&Permission::MEMBERS, result.as_slice());
    }

    #[test]
    fn reports_path_depth() {
        assert_eq!(Permission::PostView.path_depth(), 1);
        assert_eq!(Permission::PostEditTags.path_depth(), 2);
        assert_eq!(Permission::PostMoveIn.path_depth(), 2);
    }

    #[test]
    fn reports_predicate_scope() {
        assert_eq!(Permission::PostView.scope(), PredicateScope::Post);
        assert_eq!(Permission::UserInvite.scope(), PredicateScope::User);
        assert_eq!(Permission::TagImplicationEdit.scope(), PredicateScope::Tag);
    }
}
