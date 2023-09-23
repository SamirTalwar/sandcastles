use lazy_static::lazy_static;

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Name(String);

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for Name {
    type Err = (); // should be `!` but that's experimental

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

impl From<&str> for Name {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for Name {
    fn from(value: String) -> Self {
        Self(value)
    }
}

const ADJECTIVES_STR: &str = include_str!("names_adjectives.txt");
const NOUNS_STR: &str = include_str!("names_nouns.txt");

lazy_static! {
    static ref ADJECTIVES: Vec<&'static str> = ADJECTIVES_STR
        .split('\n')
        .filter(|s| !s.is_empty())
        .collect();
    static ref NOUNS: Vec<&'static str> = NOUNS_STR.split('\n').filter(|s| !s.is_empty()).collect();
}

pub fn random_name() -> Name {
    use rand::prelude::*;
    let mut rng = rand::thread_rng();
    let adjective = ADJECTIVES.choose(&mut rng).unwrap();
    let noun = NOUNS.choose(&mut rng).unwrap();
    Name(format!("{}-{}", adjective, noun))
}
