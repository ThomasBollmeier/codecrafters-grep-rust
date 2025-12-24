use anyhow::*;
use crate::matcher::{make_alpha_num_matcher, make_digit_matcher, Matcher};
use crate::matcher::Matcher::Multiple;

#[derive(Debug)]
pub struct RegexParser {
    pattern: Vec<char>,
    index: usize,
    next_group_idx: usize,
}

impl RegexParser {
    pub fn new(pattern: &str) -> RegexParser {
        Self::new_with_next_group_idx(pattern, 1)
    }

    fn new_with_next_group_idx(pattern: &str, next_group_idx: usize) -> RegexParser {
        Self {
            pattern: pattern.chars().collect(),
            index: 0,
            next_group_idx,
        }
    }

    pub fn parse(&mut self) -> Result<Matcher> {
        let mut matchers = vec![];

        while let Some(ch) = self.peek() {
            let matcher = match ch {
                '\\' =>  {
                    let next_ch = self
                        .peek_nth(1)
                        .ok_or(anyhow!("expected escaped char"))?;
                    let matcher = match next_ch {
                        'd' => make_digit_matcher(),
                        'w' => make_alpha_num_matcher(),
                        '\\' | '+' | '?' | '.' | '[' | '(' => Matcher::new_single_char(next_ch),
                        _ => {
                            if next_ch.is_ascii_digit() {
                                let group_idx: usize = next_ch.to_digit(10).unwrap() as usize;
                                if group_idx >= self.next_group_idx {
                                    return Err(anyhow!("invalid group index"));
                                }
                                Matcher::new_group_reference(group_idx)
                            } else {
                                return Err(anyhow!("Invalid character '{}'", next_ch))
                            }
                        }
                    };
                    self.advance()?;
                    self.advance()?;
                    matcher
                },
                '[' => self.parse_group_matcher()?,
                '(' => self.parse_group()?,
                '^' => {
                    self.advance()?;
                    Matcher::new_start()
                }
                '$' => {
                    self.advance()?;
                    Matcher::new_end()
                }
                '+' => {
                    self.advance()?;
                    let last_matcher = matchers
                        .iter()
                        .last()
                        .cloned()
                        .ok_or(anyhow!("+ expects previous char"))?;
                    let matcher = Matcher::new_one_or_more(
                        Box::new(last_matcher), None);
                    matchers.pop();
                    matcher
                }
                '*' => {
                    self.advance()?;
                    let last_matcher = matchers
                        .iter()
                        .last()
                        .cloned()
                        .ok_or(anyhow!("+ expects previous char"))?;
                    let matcher = Matcher::new_zero_or_more(
                        Box::new(last_matcher), None);
                    matchers.pop();
                    matcher
                }                
                '?' => {
                    self.advance()?;
                    let last_matcher = matchers
                        .iter()
                        .last()
                        .ok_or(anyhow!("+ expects previous char"))?;
                    let matcher = Matcher::new_zero_or_one(last_matcher);
                    matchers.pop();
                    matcher
                }
                '{' => {
                    let last_matcher = matchers
                        .iter()
                        .last()
                        .ok_or(anyhow!("+ expects previous char"))?;
                    let (min, max) = self.parse_quantifiers()?;
                    let matcher = Multiple {
                        matcher: Box::new(last_matcher.clone()),
                        min,
                        max,
                        follow: None
                    };
                    matchers.pop();
                    matcher
                }
                '.' => {
                    self.advance()?;
                    Matcher::new_wildcard()
                }
                _ => {
                    self.advance()?;
                    Matcher::new_single_char(ch)
                },
            };

            matchers.push(matcher);
        }

        match matchers.len() {
            0 => Err(anyhow!("No matcher found")),
            1 => Ok(matchers[0].clone()),
            _ => Ok(Matcher::new_sequence(matchers)),
        }
    }

    fn parse_quantifiers(&mut self) -> Result<(usize, Option<usize>)> {
        self.advance()?;
        let mut min_str = String::new();
        loop {
            let ch = self.advance()?;
            if ch.is_ascii_digit() {
                min_str.push(ch);
            } else if ch == '}' {
                let min = min_str.parse::<usize>()?;
                return Ok((min, Some(min)));
            } else {
                return Err(anyhow!("invalid quantifier character '{}'", ch));
            }
        }
    }

    fn parse_group(&mut self) -> Result<Matcher> {
        let (segments, consumed_len) = self.split_alternation()?;
        let mut matchers = vec![];
        let group_idx = self.next_group_idx;
        self.next_group_idx += 1;

        for segment in &segments {
            let mut parser = RegexParser::new_with_next_group_idx(segment, self.next_group_idx);
            let matcher = parser.parse()?;
            matchers.push(matcher);
            self.next_group_idx = parser.next_group_idx;
        }
        self.index += consumed_len;

        Ok(Matcher::new_group(matchers, group_idx))
    }

    fn split_alternation(&self) -> Result<(Vec<String>, usize)> {
        let mut segments = vec![];
        let mut segment = String::new();
        let mut level = 0;
        let mut consumed_len = 0;

        for (idx, ch) in self.pattern[self.index..].iter().enumerate() {
            match *ch {
                '(' => {
                    level += 1;
                    if level == 1 {
                        continue;
                    }
                },
                ')' => {
                    level -= 1;
                    if level == 0 {
                        if segment.is_empty() {
                            return Err(anyhow!("Empty alternation"));
                        }
                        segments.push(segment.clone());
                        segment.clear();
                        consumed_len = idx + 1;
                        break;
                    }
                },
                '|' => {
                    if level == 1 {
                        if segment.is_empty() {
                            return Err(anyhow!("Empty alternation"));
                        }
                        segments.push(segment.clone());
                        segment.clear();
                        continue;
                    }
                }
                _ => {}
            }
            segment.push(*ch);
        }

        if !segment.is_empty() || level != 0 {
            return Err(anyhow!("Invalid alternation pattern"));
        }

        Ok((segments, consumed_len))
    }

    fn parse_group_matcher(&mut self) -> Result<Matcher> {
        let mut characters = vec![];
        let mut is_negated = false;

        self.advance()?;
        let ch = self.peek().ok_or(anyhow!("expected character"))?;
        if ch == '^' {
            is_negated = true;
        } else {
            characters.push(ch);
        }
        self.advance()?;

        loop {
            let ch = self.advance()?;
            if ch == ']' {
                break;
            }
            characters.push(ch);
        }

        Ok(Matcher::new_single_char_branch(characters, is_negated))
    }

    fn advance(&mut self) -> Result<char> {
        if self.index >= self.pattern.len() {
            return Err(anyhow!("End of pattern reached"));
        }
        let ch = self.pattern[self.index];
        self.index += 1;

        Ok(ch)
    }

    fn peek(&self) -> Option<char> {
        if self.index >= self.pattern.len() {
            return None;
        }
        Some(self.pattern[self.index])
    }

    fn peek_nth(&self, n: usize) -> Option<char> {
        if self.index + n >= self.pattern.len() {
            return None;
        }
        Some(self.pattern[self.index + n])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_matcher(pattern: &str) -> Matcher {
        RegexParser::new(pattern).parse().unwrap()
    }

    #[test]
    fn test_single_char_matcher() {
        let matcher = make_matcher("a");
        let m = matcher.find_match("cat");
        assert!(m.is_some());

        let m = m.unwrap();
        assert_eq!(m.offset, 1);
    }

    #[test]
    fn test_negative_group() {
        let text = "banana";
        let matcher = make_matcher("[^anb]");

        assert!(matcher.find_match(text).is_none());
    }

    #[test]
    fn test_sequence_matcher() {
        let text = "sally has 3 apples";
        let pattern = r"\d apples";

        let matcher = make_matcher(pattern);
        let m_opt = matcher.find_match(text);

        assert!(m_opt.is_some());
    }

    #[test]
    fn sequence_matcher_works() {
        let text = "babanana";
        let pattern = "ban";

        let matcher = make_matcher(pattern);
        let m_opt = matcher.find_match(text);

        assert!(m_opt.is_some());
        let m = m_opt.unwrap();
        assert_eq!(m.matched_text, "ban");
        assert_eq!(m.offset, 2);
    }

    #[test]
    fn test_start_matcher() {
        let start_matcher = make_matcher("^ban");

        let m = start_matcher.find_match("rayban");
        assert!(m.is_none());
        let m = start_matcher.find_match("banner");
        assert!(m.is_some());
        assert_eq!(m.unwrap().offset, 0);
    }

    #[test]
    fn test_end_matcher() {
        let end_matcher = make_matcher("ban$");

        let m = end_matcher.find_match("banner");
        assert!(m.is_none());
        let m = end_matcher.find_match("rayban");
        assert!(m.is_some());
        assert_eq!(m.unwrap().offset, 3);
    }

    #[test]
    fn test_one_or_more_matcher() {
        let matcher = make_matcher("o+");
        let m = matcher.find_match("room");
        assert!(m.is_some());
        assert_eq!(m.unwrap().matched_text, "oo");
    }

    #[test]
    fn test_one_or_more_matcher_not_greedy() {
        let matcher = make_matcher("ca+at");
        let m = matcher.find_match("caaats");
        assert!(m.is_some());
        assert_eq!(m.unwrap().matched_text, "caaat");
    }

    #[test]
    fn test_one_or_more_matcher_at_start() {
        let matcher = make_matcher(".+berry");
        let m = matcher.find_match("strawberry");
        assert!(m.is_some());
        assert_eq!(m.unwrap().matched_text, "strawberry");
    }

    #[test]
    fn test_zero_or_one_matcher() {
        let matcher = make_matcher("re?m");
        let m = matcher.find_match("rm");
        assert!(m.is_some());
        let m = matcher.find_match("rm");
        assert!(m.is_some());
    }

    #[test]
    fn test_zero_or_more_matcher() {
        let matcher = make_matcher("go*gle");
        let m = matcher.find_match("ggler");
        assert!(m.is_some());
        assert_eq!(m.unwrap().matched_text, "ggle");
    }

    #[test]
    fn test_quantifier() {
        let matcher = make_matcher("ro{2}m");
        let m = matcher.find_match("rome");
        assert!(m.is_none());
        let m = matcher.find_match("room");
        assert!(m.is_some());
        let m = matcher.find_match("vroooom");
        assert!(m.is_none());
    }

    #[test]
    fn test_wildcard_matcher() {
        let matcher = make_matcher("g.+gol");
        let m = matcher.find_match("goøö0Ogol");
        assert!(m.is_some());
    }

    #[test]
    fn test_alternation_matcher() {
        let matcher = make_matcher("(pad|r(a|ö))deln");
        let m = matcher.find_match("paddeln");
        assert!(m.is_some());
        let m = matcher.find_match("radeln");
        assert!(m.is_some());
        let m = matcher.find_match("rodeln");
        assert!(m.is_none());
        let m = matcher.find_match("rödeln");
        assert!(m.is_some());
    }

    #[test]
    fn test_single_backreference_matcher() {
        let matcher = make_matcher(r"(\w+) and \1");
        let m = matcher.find_match("cat and cat");
        assert!(m.is_some());
        let m = matcher.find_match("cat and dog");
        assert!(m.is_none());
    }

    #[test]
    fn test_multiple_backreference_matcher() {
        let matcher = make_matcher(r"(\d+) (\w+) squares and \1 \2 circles");
        let m = matcher.find_match("3 red squares and 3 red circles");
        assert!(m.is_some());
        let m = matcher.find_match("3 red squares and 3 blue circles");
        assert!(m.is_none());
    }

    #[test]
    fn test_nested_backreference_matcher() {
        let matcher = make_matcher(r"(([abc]+)-([def]+)) is \1, not ([^xyz]+), \2, or \3");
        let m = matcher.find_match("abc-def is abc-def, not efg, abc, or def");
        assert!(m.is_some());
    }
}