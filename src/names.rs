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
    type Err = NameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

#[derive(Debug)]
pub enum NameError {}

impl std::fmt::Display for NameError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match *self {}
    }
}

impl std::error::Error for NameError {}

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
