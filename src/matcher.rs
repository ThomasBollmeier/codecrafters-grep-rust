#[derive(Clone, Debug)]
pub enum Matcher {
    SingleChar(char),
    StartMatcher,
    EndMatcher,
    SingleCharBranch(Vec<char>, bool),
    Sequence(Vec<Matcher>),
    OneOrMore{
        matcher: Box<Matcher>,
        follow: Option<Box<Matcher>>,
    },
    ZeroOrOne(Box<Matcher>),
    Wildcard,
    Alternation(Vec<Matcher>),
}

impl Matcher {

    pub fn new_single_char(c: char) -> Self {
        Matcher::SingleChar(c)
    }

    pub fn new_start() -> Self {
        Matcher::StartMatcher
    }

    pub fn new_end() -> Self {
        Matcher::EndMatcher
    }

    pub fn new_single_char_branch(chars: Vec<char>, negated: bool) -> Self {
        Matcher::SingleCharBranch(chars, negated)
    }

    pub fn new_sequence(matchers: Vec<Matcher>) -> Self {
        Matcher::Sequence(matchers)
    }

    pub fn new_one_or_more(matcher: Box<Matcher>, follow: Option<&Matcher>) -> Self {
        Matcher::OneOrMore{
            matcher,
            follow: match follow {
                Some(f) => Some(Box::new(f.clone())),
                None => None,
            },
        }
    }

    pub fn new_zero_or_one(matcher: &Matcher) -> Self {
        Matcher::ZeroOrOne(Box::new(matcher.clone()))
    }

    pub fn new_wildcard() -> Self {
        Matcher::Wildcard
    }

    pub fn new_alternation(matchers: Vec<Matcher>) -> Self {
        Matcher::Alternation(matchers)
    }

    pub fn matches(&self, text: &str) -> bool {
        self.find_match(text).is_some()
    }

    pub fn find_match(&self, text: &str) -> Option<Match> {
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

    pub fn check_match(&self, text: &str, offset: usize) -> Option<String> {
        use Matcher::*;
        match self {
            SingleChar(ch) => self.check_single_char(*ch, text, offset),
            StartMatcher => self.check_start(text, offset),
            EndMatcher => self.check_end(text, offset),
            SingleCharBranch(characters, is_negated) =>
                self.check_single_char_branch(characters, *is_negated, text, offset),
            Sequence(matchers) => self.check_sequence(matchers, text, offset),
            OneOrMore{
                matcher,
                follow
            } => {
                self.check_one_or_more(matcher, follow, text, offset)
            },
            ZeroOrOne(matcher) => self.check_zero_or_one(matcher, text, offset),
            Wildcard => self.check_wildcard(text, offset),
            Alternation(matchers) => self.check_alternation(matchers, text, offset),
        }
    }

    fn check_single_char(&self, ch: char, text: &str, offset: usize) -> Option<String> {
        if offset >= text.chars().count() {
            return None;
        }
        let c = text.chars().nth(offset).unwrap();
        if c == ch {
            Some(ch.to_string())
        } else {
            None
        }
    }

    fn check_start(&self, _text: &str, offset: usize) -> Option<String> {
        if offset == 0 {
            Some("".to_string())
        } else {
            None
        }
    }

    fn check_end(&self, text: &str, offset: usize) -> Option<String> {
        if offset == text.len() {
            Some("".to_string())
        } else {
            None
        }
    }

    fn check_single_char_branch(&self,
                                characters: &Vec<char>,
                                is_negated: bool,
                                text: &str,
                                offset: usize) -> Option<String> {

        if !is_negated {
            match text.chars().nth(offset) {
                Some(ch) => {
                    for c in characters {
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
                    for c in characters {
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

    fn check_sequence(&self, elements: &Vec<Matcher>, text: &str, offset: usize) -> Option<String> {
        let mut curr_offset = offset;
        let mut matched_text = String::new();

        for element in elements {
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

    fn check_one_or_more(&self,
                         matcher: &Matcher,
                         follow: &Option<Box<Matcher>>,
                         text: &str,
                         offset: usize) -> Option<String> {

        let mut curr_offset = offset;
        let mut matched_text = String::new();
        loop {
            match matcher.check_match(text, curr_offset) {
                Some(m_text) => {
                    // If there is a following matcher that matches
                    // stop matching to avoid "greedy" matching behavior
                    if !matched_text.is_empty() &&
                        follow.is_some() &&
                        follow.as_ref().unwrap().matches(&m_text) {
                        return Some(matched_text);
                    }
                    matched_text.push_str(&m_text);
                    curr_offset += m_text.chars().count();
                }
                None => if matched_text.is_empty() {
                    return None;
                } else {
                    break;
                },
            }
        }

        Some(matched_text)
    }

    fn check_zero_or_one(&self, matcher: &Matcher, text: &str, offset: usize) -> Option<String> {
        let mut matched_text = String::new();
        match matcher.check_match(text, offset) {
            Some(m_text) => {
                matched_text.push_str(&m_text);
            }
            None => {}
        }

        Some(matched_text)
    }

    fn check_wildcard(&self, text: &str, offset: usize) -> Option<String> {
        text.chars().nth(offset).map(|c| c.to_string())
    }

    fn check_alternation(&self, matchers: &Vec<Matcher>, text: &str, offset: usize) -> Option<String> {
        for matcher in matchers {
            if let Some(s) = matcher.check_match(text, offset) {
                return Some(s);
            }
        }
        None
    }
}

pub struct Match {
    pub matched_text: String,
    pub offset: usize,
}

pub fn make_digit_matcher() -> Matcher {
    let digits = vec!['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
    Matcher::new_single_char_branch(digits, false)
}

pub fn make_alpha_num_matcher() -> Matcher {
    let lower_chars = "abcdefghijklmnopqrstuvwxyz";
    let upper_chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let digits = "0123456789";

    let mut alpha_nums = lower_chars.to_string();
    alpha_nums.push_str(&upper_chars);
    alpha_nums.push_str(&digits);
    alpha_nums.push('_');

    Matcher::new_single_char_branch(alpha_nums.chars().collect(), false)
}

pub fn make_group_matcher(pattern: &str) -> Matcher {
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

    Matcher::new_single_char_branch(
        characters,
        is_negated,
    )
}