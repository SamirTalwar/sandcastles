use lazy_static::lazy_static;

const ADJECTIVES_STR: &str = include_str!("names_adjectives.txt");
const NOUNS_STR: &str = include_str!("names_nouns.txt");

lazy_static! {
    static ref ADJECTIVES: Vec<&'static str> = ADJECTIVES_STR
        .split('\n')
        .filter(|s| !s.is_empty())
        .collect();
    static ref NOUNS: Vec<&'static str> = NOUNS_STR.split('\n').filter(|s| !s.is_empty()).collect();
}

pub fn random_name() -> String {
    use rand::prelude::*;
    let mut rng = rand::thread_rng();
    let adjective = ADJECTIVES.choose(&mut rng).unwrap();
    let noun = NOUNS.choose(&mut rng).unwrap();
    format!("{}-{}", adjective, noun)
}
