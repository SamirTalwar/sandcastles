use lazy_static::lazy_static;

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Name(String);

impl Name {
    const MAX_LENGTH: usize = 63;

    const VALID_STARTING_CHARACTERS: [char; 52] = [
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
        'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j',
        'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];
    const VALID_CHARACTERS: [char; 64] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H',
        'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '_', '-',
    ];
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for Name {
    type Err = NameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > Self::MAX_LENGTH {
            return Err(NameError::TooLong(s.to_owned()));
        }
        let mut chars = s.chars();
        match chars.next() {
            None => {
                return Err(NameError::EmptyName);
            }
            Some(first_char) => {
                if !Self::VALID_STARTING_CHARACTERS.contains(&first_char) {
                    return Err(NameError::InvalidName(s.to_owned()));
                }
            }
        }
        for char in chars {
            if !Self::VALID_CHARACTERS.contains(&char) {
                return Err(NameError::InvalidName(s.to_owned()));
            }
        }
        Ok(Self(s.to_owned()))
    }
}

#[derive(Debug, PartialEq)]
pub enum NameError {
    EmptyName,
    TooLong(String),
    InvalidName(String),
}

impl std::fmt::Display for NameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            NameError::EmptyName => write!(f, "a name cannot be empty"),
            NameError::TooLong(name) => write!(f, "name too long: {:?}, the name must be at most {} characters", name, Name::MAX_LENGTH),
            NameError::InvalidName(name) => write!(f, "invalid name: {:?}, the name must contain only letters, numbers, hyphens, and underscores, and start with a letter", name),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_allows_alphanumeric_hyphens_and_underscores() {
        let name = "abc-DEF_123-ghi".parse::<Name>();

        assert_eq!(name, Ok(Name("abc-DEF_123-ghi".to_owned())));
    }

    #[test]
    fn test_name_rejects_an_empty_name() {
        let name = "".parse::<Name>();

        assert_eq!(name, Err(NameError::EmptyName));
    }

    #[test]
    fn test_name_rejects_a_name_that_is_too_long() {
        // this name has 64 characters
        let name =
            "abcdefghij0123456789abcdefghij0123456789abcdefghij0123456789abcd".parse::<Name>();

        assert_eq!(
            name,
            Err(NameError::TooLong(
                "abcdefghij0123456789abcdefghij0123456789abcdefghij0123456789abcd".to_owned()
            ))
        );
    }

    #[test]
    fn test_name_rejects_spaces() {
        let name = "a b c".parse::<Name>();

        assert_eq!(name, Err(NameError::InvalidName("a b c".to_owned())));
    }

    #[test]
    fn test_name_rejects_accents() {
        let name = "á".parse::<Name>();

        assert_eq!(name, Err(NameError::InvalidName("á".to_owned())));
    }

    #[test]
    fn test_name_rejects_hyphens_at_the_start() {
        let name = "-abc".parse::<Name>();

        assert_eq!(name, Err(NameError::InvalidName("-abc".to_owned())));
    }

    #[test]
    fn test_name_rejects_underscores_at_the_start() {
        let name = "_def".parse::<Name>();

        assert_eq!(name, Err(NameError::InvalidName("_def".to_owned())));
    }

    #[test]
    fn test_name_rejects_numbers_at_the_start() {
        let name = "9ghi".parse::<Name>();

        assert_eq!(name, Err(NameError::InvalidName("9ghi".to_owned())));
    }
}
