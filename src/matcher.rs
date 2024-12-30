use std::collections::HashMap;

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
    Group(Vec<Matcher>, usize),
    GroupReference(usize),
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

    pub fn new_group(matchers: Vec<Matcher>, group_idx: usize) -> Self {
        Matcher::Group(matchers, group_idx)
    }

    pub fn new_group_reference(group_idx: usize) -> Self {
        Matcher::GroupReference(group_idx)
    }

    pub fn matches(&self, text: &str) -> bool {
        self.find_match(text).is_some()
    }

    pub fn find_match(&self, text: &str) -> Option<Match> {
        for offset in 0..text.chars().count() {
            match self.check_match(text, offset, &HashMap::new()) {
                Some(m) => return Some(m),
                None => continue,
            }
        }
        None
    }

    fn check_match(&self,
                   text: &str,
                   offset: usize,
                   group_results: &HashMap<usize, String>) -> Option<Match> {

        use Matcher::*;
        match self {
            SingleChar(ch) => self.check_single_char(*ch, text, offset),
            StartMatcher => self.check_start(text, offset),
            EndMatcher => self.check_end(text, offset),
            SingleCharBranch(characters, is_negated) =>
                self.check_single_char_branch(characters, *is_negated, text, offset),
            Sequence(matchers) =>
                self.check_sequence(matchers, text, offset, group_results),
            OneOrMore{ matcher, follow } =>
                self.check_one_or_more(matcher, follow, text, offset, group_results),
            ZeroOrOne(matcher) => self.check_zero_or_one(matcher, text, offset, group_results),
            Wildcard => self.check_wildcard(text, offset),
            Group(matchers, group_idx) =>
                self.check_group(matchers, *group_idx, text, offset, group_results),
            GroupReference(group_idx) =>
                self.check_group_reference(*group_idx, text, offset, group_results),
        }
    }

    fn check_single_char(&self, ch: char, text: &str, offset: usize) -> Option<Match> {
        if offset >= text.chars().count() {
            return None;
        }
        let c = text.chars().nth(offset).unwrap();
        if c == ch {
            Some(Match::new(&ch.to_string(), offset))
        } else {
            None
        }
    }

    fn check_start(&self, _text: &str, offset: usize) -> Option<Match> {
        if offset == 0 {
            Some(Match::new("", offset))
        } else {
            None
        }
    }

    fn check_end(&self, text: &str, offset: usize) -> Option<Match> {
        if offset == text.len() {
            Some(Match::new("", offset))
        } else {
            None
        }
    }

    fn check_single_char_branch(&self,
                                characters: &Vec<char>,
                                is_negated: bool,
                                text: &str,
                                offset: usize) -> Option<Match> {

        if !is_negated {
            match text.chars().nth(offset) {
                Some(ch) => {
                    for c in characters {
                        if *c == ch {
                            return Some(Match::new(&ch.to_string(), offset));
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
                    Some(Match::new(&ch.to_string(), offset))
                }
                None => None,
            }
        }
    }

    fn check_sequence(&self,
                      elements: &Vec<Matcher>,
                      text: &str,
                      offset: usize,
                      group_results:&HashMap<usize, String>) -> Option<Match> {
        let mut curr_offset = offset;
        let mut curr_groups = group_results.clone();
        let mut m = Match::new("", offset);

        for element in elements {
            match element.check_match(text, curr_offset, &curr_groups) {
                Some(other) => {
                    m.accumulate(&other);
                    curr_offset += other.matched_text.chars().count();
                    Self::update_group_results(&mut curr_groups, &other);
                }
                None => return None,
            }
        }
        Some(m)
    }

    fn update_group_results(group_results: &mut HashMap<usize, String>, m: &Match) {
        for (gid, sub_match) in &m.sub_matches {
            group_results.insert(*gid, sub_match.matched_text.clone());
        }
    }

    fn check_one_or_more(&self,
                         matcher: &Matcher,
                         follow: &Option<Box<Matcher>>,
                         text: &str,
                         offset: usize,
                         group_results: &HashMap<usize, String>) -> Option<Match> {

        let mut curr_offset = offset;
        let mut m = Match::new("", offset);
        let mut curr_groups = group_results.clone();

        loop {
            match matcher.check_match(text, curr_offset, &curr_groups) {
                Some(other) => {
                    // If there is a following matcher that matches
                    // stop matching to avoid "greedy" matching behavior
                    if !m.matched_text.is_empty() &&
                        follow.is_some() &&
                        follow.as_ref().unwrap().matches(&other.matched_text) {
                        return Some(m);
                    }

                    m.accumulate(&other);
                    curr_offset += other.matched_text.chars().count();
                    Self::update_group_results(&mut curr_groups, &other);
                }
                None => if m.matched_text.is_empty() {
                    return None;
                } else {
                    break;
                },
            }
        }

        Some(m)
    }

    fn check_zero_or_one(&self,
                         matcher: &Matcher,
                         text: &str,
                         offset: usize,
                         group_results: &HashMap<usize, String>) -> Option<Match> {

        let mut m = Match::new("", offset);
        match matcher.check_match(text, offset, group_results) {
            Some(other) => {
                m.accumulate(&other);
            }
            None => {}
        }

        Some(m)
    }

    fn check_wildcard(&self, text: &str, offset: usize) -> Option<Match> {
        text.chars().nth(offset).map(|c| Match::new(&c.to_string(), offset))
    }

    fn check_group(&self,
                   matchers: &Vec<Matcher>,
                   group_idx: usize,
                   text: &str,
                   offset: usize,
                   group_results: &HashMap<usize, String>) -> Option<Match> {

        let mut group_match = Match::new("", offset);

        for matcher in matchers {
            if let Some(m) = matcher.check_match(text, offset, group_results) {
                group_match.accumulate(&m);
                group_match.sub_matches.insert(group_idx, group_match.clone());
                return Some(group_match);
            }
        }
        None
    }

    fn check_group_reference(&self,
                             group_idx: usize,
                             text: &str,
                             offset: usize,
                             group_results: &HashMap<usize, String>) -> Option<Match> {

        match group_results.get(&group_idx) {
            Some(matched) => {
                let text = text.chars().skip(offset).collect::<String>();
                if text.starts_with(matched) {
                    Some(Match::new(&matched, offset))
                } else {
                    None
                }
            }
            None => None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Match {
    pub matched_text: String,
    pub offset: usize,
    pub sub_matches: HashMap<usize, Match>,
}

impl Match {
    fn new(matched_text: &str, offset: usize) -> Self {
        Self {
            matched_text: matched_text.to_string(),
            offset,
            sub_matches: HashMap::new(),
        }
    }

    fn accumulate(&mut self, other: &Match) {
        self.matched_text.push_str(&other.matched_text);
        for (group_idx, sub_match) in &other.sub_matches {
            self.sub_matches.insert(*group_idx, sub_match.clone());
        }
    }

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