use anyhow::*;
use crate::matcher::{make_alpha_num_matcher, make_digit_matcher, Matcher};

pub struct RegexParser {
    pattern: Vec<char>,
    index: usize,
}

impl RegexParser {
    pub fn new(pattern: &str) -> RegexParser {
        Self { pattern: pattern.chars().collect(), index: 0 }
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
                        '\\' => Matcher::new_single_char(next_ch),
                        _ => return Err(anyhow!("Invalid character '{}'", next_ch)),
                    };
                    self.advance()?;
                    self.advance()?;
                    matcher
                },
                '[' => self.parse_group_matcher()?,
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
                        .ok_or(anyhow!("+ expects previous char"))?;
                    let matcher = Matcher::new_one_or_more(last_matcher);
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
    fn test_zero_or_more_matcher() {
        let matcher = make_matcher("re?m");
        let m = matcher.find_match("rm");
        assert!(m.is_some());
        let m = matcher.find_match("rm");
        assert!(m.is_some());
    }
}