use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Matcher {
    SingleChar(char),
    StartMatcher,
    EndMatcher,
    SingleCharBranch(Vec<char>, bool),
    Sequence(Vec<Matcher>),
    Multiple{
        matcher: Box<Matcher>,
        min: usize,
        max: Option<usize>,
        follow: Option<Box<Matcher>>,
    },
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
        let mut new_matchers = vec![];
        let mut pending: Option<Matcher> = None;

        for matcher in matchers {
            if let Some(p) = &pending {
                if p.is_mergeable_with(&matcher) {
                    // Merge the two matchers
                    let merged = p.merge_with(&matcher);
                    pending = Some(merged);
                } else {
                    // Push the pending matcher to new_matchers
                    if !p.can_have_follow() {
                        new_matchers.push(p.clone());
                    } else {
                        new_matchers.push(p.set_follow(&matcher));
                    }
                    pending = Some(matcher);
                }
            } else {
                pending = Some(matcher);
            }
        }

        if let Some(p) = pending {
            new_matchers.push(p);
        }

        Matcher::Sequence(new_matchers)
    }

    pub fn new_one_or_more(matcher: Box<Matcher>, follow: Option<&Matcher>) -> Self {
        Matcher::Multiple{
            matcher,
            min: 1,
            max: None,
            follow: match follow {
                Some(f) => Some(Box::new(f.clone())),
                None => None,
            },
        }
    }

    pub fn new_zero_or_more(matcher: Box<Matcher>, follow: Option<&Matcher>) -> Self {
        Matcher::Multiple{
            matcher,
            min: 0,
            max: None,
            follow: match follow {
                Some(f) => Some(Box::new(f.clone())),
                None => None,
            },
        }
    }

    pub fn new_zero_or_one(matcher: &Matcher) -> Self {
        Matcher::Multiple{
            matcher: Box::new(matcher.clone()),
            min: 0,
            max: Some(1),
            follow: None,
        }
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

    fn is_mergeable_with(&self, other: &Matcher) -> bool {
        use Matcher::*;
        match (self, other) {
            (Multiple { matcher: m1, ..}, Multiple{matcher: m2, ..}) => {
                **m1 == **m2
            }
            (Multiple { matcher: m1, ..}, _) => {
                **m1 == *other
            }
            _ => false,
        }
    }

    fn merge_with(&self, other: &Matcher) -> Matcher {
        use Matcher::*;

        if !self.is_mergeable_with(other) {
            panic!("Cannot merge non-mergeable matchers");
        }

        match (self, other) {
            (Multiple { matcher: m, min: min1, max: max1, .. },
             Multiple { min: min2, max: max2, .. }) => {
                let new_min = min1 + min2;
                let new_max = match (max1, max2) {
                    (Some(v1), Some(v2)) => Some(v1 + v2),
                    _ => None,
                };
                Multiple {
                    matcher: m.clone(),
                    min: new_min,
                    max: new_max,
                    follow: None,
                }
            }
            (Multiple { matcher: m, min, max, .. }, _) => {
                let new_min = min + 1;
                let new_max = match max {
                    Some(v) => Some(v + 1),
                    None => None,
                };
                Multiple {
                    matcher: m.clone(),
                    min: new_min,
                    max: new_max,
                    follow: None,
                }
            }
            _ => panic!("Cannot merge non-mergeable matchers"),
        }
    }

    fn can_have_follow(&self) -> bool {
        match self {
            Matcher::Multiple {..} => true,
            Matcher::Group(matchers,_) => {
                let last_match = matchers.last().unwrap();
                last_match.can_have_follow()
            },
            _ => false,
        }
    }

    fn set_follow(&self, follow: &Matcher) -> Matcher  {
        match self {
            Matcher::Multiple { matcher, min, max,.. } => Matcher::Multiple {
                matcher: matcher.clone(),
                min: *min,
                max: *max,
                follow: Some(Box::new(follow.clone())),
            },
            Matcher::Group(matchers, group_idx) => {
                let last_matcher = matchers.last().unwrap();
                let new_last_matcher = last_matcher.set_follow(follow);
                let mut new_matchers = matchers.clone();
                new_matchers.pop();
                new_matchers.push(new_last_matcher);
                Matcher::Group(new_matchers, *group_idx)
            }
            _ => panic!("set_follow can only be called on OneOrMore or ZeroOrMore matchers"),
        }
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
            Multiple { matcher, min, max, follow } =>
                self.check_multiple(matcher, *min, *max, follow, text, offset, group_results),
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
                None => return None
            }
        }
        Some(m)
    }

    fn update_group_results(group_results: &mut HashMap<usize, String>, m: &Match) {
        for (gid, sub_match) in &m.sub_matches {
            group_results.insert(*gid, sub_match.matched_text.clone());
        }
    }

    fn check_multiple(&self,
                      matcher: &Matcher,
                      min: usize,
                      max: Option<usize>,
                      follow: &Option<Box<Matcher>>,
                      text: &str,
                      offset: usize,
                      group_results: &HashMap<usize, String>) -> Option<Match> {
        let mut curr_offset = offset;
        let mut m = Match::new("", offset);
        let mut curr_groups = group_results.clone();
        let mut count = 0;

        loop {
            let min_reached = count >= min;
            let max_reached = match max {
                Some(max_val) => count >= max_val,
                None => true,
            };

            match matcher.check_match(text, curr_offset, &curr_groups) {
                Some(other) => {
                    // If there is a following matcher that matches
                    // stop matching to avoid "greedy" matching behavior
                    if min_reached && max_reached && follow.is_some() &&
                        follow.as_ref().unwrap().matches(&other.matched_text) {
                        return Some(m);
                    }

                    m.accumulate(&other);
                    curr_offset += other.matched_text.chars().count();
                    Self::update_group_results(&mut curr_groups, &other);
                    count += 1;

                    if let Some(max_val) = max {
                        if count >= max_val {
                            return Some(m);
                        }
                    }

                }
                None => {
                    return if min_reached {
                        Some(m)
                    } else {
                        None
                    }
                }
            }
        }
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
