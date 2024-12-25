use anyhow::*;

pub trait Matcher {
    fn matches(&self, text: &str) -> bool {
        self.find_match(text).is_some()
    }

    fn check_match(&self, text: &str, offset: usize) -> Option<String>;

    fn find_match(&self, text: &str) -> Option<Match> {
        for offset in 0..text.chars().count() {
            match self.check_match(text, offset) {
                Some(matched_text) => return Some(Match {
                    matched_text,
                    offset,
                }),
                None => continue,
            }
        }
        None
    }
}

pub struct Match {
    pub matched_text: String,
    pub offset: usize,
}

pub struct SingleCharMatcher {
    ch: char,
}

impl SingleCharMatcher {
    pub fn new(ch: char) -> Self {
        Self { ch }
    }
}

impl Matcher for SingleCharMatcher {
    fn matches(&self, text: &str) -> bool {
        text.contains(self.ch)
    }

    fn check_match(&self, text: &str, offset: usize) -> Option<String> {
        if offset >= text.len() {
            return None;
        }
        let ch = text.chars().nth(offset).unwrap();
        if ch == self.ch {
            Some(ch.to_string())
        } else {
            None
        }
    }
}

pub struct StartMatcher {}

impl StartMatcher {
    pub fn new() -> Self {
        Self {}
    }
}

impl Matcher for StartMatcher {
    fn check_match(&self, _text: &str, offset: usize) -> Option<String> {
        if offset == 0 {
            Some("".to_string())
        } else {
            None
        }
    }
}

pub struct SingleCharBranchMatcher {
    characters: Vec<char>,
    is_negated: bool,
}

impl SingleCharBranchMatcher {
    pub fn new(characters: Vec<char>, is_negated: bool) -> Self {
        Self { characters, is_negated }
    }
}

impl Matcher for SingleCharBranchMatcher {
    fn check_match(&self, text: &str, offset: usize) -> Option<String> {
        if !self.is_negated {
            match text.chars().nth(offset) {
                Some(ch) => {
                    for c in &self.characters {
                        if *c == ch {
                            return Some(ch.to_string());
                        }
                    }
                    None
                }
                None => None,
            }
        } else {
            match text.chars().nth(offset) {
                Some(ch) => {
                    for c in &self.characters {
                        if *c == ch {
                            return None;
                        }
                    }
                    Some(ch.to_string())
                }
                None => None,
            }
        }
    }
}

pub fn make_digit_matcher() -> impl Matcher {
    let digits = vec!['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
    SingleCharBranchMatcher::new(
        digits,
        false,
    )
}

pub fn make_alpha_num_matcher() -> impl Matcher {
    let lower_chars = "abcdefghijklmnopqrstuvwxyz";
    let upper_chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let digits = "0123456789";

    let mut alpha_nums = lower_chars.to_string();
    alpha_nums.push_str(&upper_chars);
    alpha_nums.push_str(&digits);
    alpha_nums.push('_');

    SingleCharBranchMatcher::new(
        alpha_nums
            .chars()
            .collect::<Vec<_>>(),
        false,
    )
}

pub fn make_group_matcher(pattern: &str) -> impl Matcher {
    if pattern.chars().count() < 2 {
        panic!("Pattern must have at least two characters");
    }

    let is_negated = pattern.chars().nth(1).unwrap() == '^';

    let characters = if !is_negated {
        let num_chars = pattern.chars().count() - 2;
        pattern
            .chars()
            .skip(1)
            .take(num_chars)
            .collect::<Vec<_>>()
    } else {
        let num_chars = pattern.chars().count() - 3;
        pattern
            .chars()
            .skip(2)
            .take(num_chars)
            .collect::<Vec<_>>()
    };

    SingleCharBranchMatcher::new(
        characters,
        is_negated,
    )
}

pub struct SequenceMatcher {
    elements: Vec<Box<dyn Matcher>>,
}

impl SequenceMatcher {
    pub fn from_pattern(pattern: &str) -> Result<Self> {
        let mut elements = vec![];
        let characters = pattern.chars().collect::<Vec<_>>();
        let n = characters.len();
        let mut i = 0;

        while i < n {
            let ch = characters[i];
            let matcher: Box<dyn Matcher>;
            match ch {
                '\\' => {
                    if i + 1 < n {
                        let next_ch = characters[i + 1];
                        match next_ch {
                            'd' => matcher = Box::new(make_digit_matcher()),
                            'w' => matcher = Box::new(make_alpha_num_matcher()),
                            '\\' => matcher = Box::new(SingleCharMatcher::new(next_ch)),
                            _ => return Err(anyhow!("Invalid character '{}'", next_ch)),
                        }
                        i += 2;
                    } else {
                        return Err(anyhow!("invalid pattern"));
                    }
                }
                '^' => if i == 0 {
                    matcher = Box::new(StartMatcher::new());
                    i += 1;
                } else {
                    return Err(anyhow!("^ must start the pattern"));
                }
                _ => {
                    matcher = Box::new(SingleCharMatcher::new(ch));
                    i += 1;
                },
            }
            elements.push(matcher);
        }

        Ok(Self { elements })
    }
}

impl Matcher for SequenceMatcher {
    fn check_match(&self, text: &str, offset: usize) -> Option<String> {
        let mut curr_offset = offset;
        let mut matched_text = String::new();

        for element in &self.elements {
            match element.check_match(text, curr_offset) {
                Some(m_text) => {
                    matched_text.push_str(&m_text);
                    curr_offset += m_text.chars().count();
                }
                None => return None,
            }
        }
        Some(matched_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negative_group() {
        let text = "banana";
        let matcher = make_group_matcher("[^anb]");

        assert!(matcher.find_match(text).is_none());
    }

    #[test]
    fn test_sequence_matcher() {
        let text = "sally has 3 apples";
        let pattern = r"\d apples";

        let matcher = SequenceMatcher::from_pattern(pattern).unwrap();
        let m_opt = matcher.find_match(text);

        assert!(m_opt.is_some());
    }

    #[test]
    fn sequence_matcher_works() {
        let text = "babanana";
        let pattern = "ban";

        let matcher = SequenceMatcher::from_pattern(pattern).unwrap();
        let m_opt = matcher.find_match(text);

        assert!(m_opt.is_some());
        let m = m_opt.unwrap();
        assert_eq!(m.matched_text, "ban");
        assert_eq!(m.offset, 2);
    }
}