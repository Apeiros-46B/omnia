use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Rating {
    Safe,
    Risky,
    Unsafe,
}

impl FromStr for Rating {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "safe" => Ok(Rating::Safe),
            "risky" => Ok(Rating::Risky),
            "unsafe" => Ok(Rating::Unsafe),
            _ => Err(()),
        }
    }
}

/// not a primary key, transient derived integer
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Collection(pub u32);
