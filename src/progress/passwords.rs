use super::*;

pub struct Passwords {
    pub(super) words: Vec<String>,
}

impl Passwords {
    pub fn load(resources: &crate::resources::Resources) -> Result<Self, String> {
        let text = String::from_utf8_lossy(resources.get("PASSWORD.TXT")?).into_owned();
        let words = text
            .lines()
            .map(|l| l.trim().to_uppercase())
            .filter(|l| !l.is_empty())
            .collect();
        Ok(Self { words })
    }

    /// The password that opens `level` (puzzle 1 has none).
    pub fn word(&self, level: u16) -> Option<&str> {
        self.words
            .get(usize::from(level).checked_sub(2)?)
            .map(String::as_str)
    }

    /// The "New Password" for the puzzle after `solved`, carrying `score`.
    pub fn generate(&self, solved: u16, score: i32) -> Option<String> {
        let word = self.word(solved + 1)?;
        let score = score.max(0) as u64;
        let hex = radix(score + 0x100000, 16);
        let code = format!("-{}{}", &hex[1..], radix(score * weight(word), 34));
        Some(format!(
            "{word}{}",
            code.replace('0', "Z").replace('O', "Y")
        ))
    }

    /// The puzzle a password opens and its score, if the score code checks
    /// out (decode_level_password, 1000:2900). `Err(level)` means the word
    /// was right but the score code was not.
    pub fn decode(&self, password: &str) -> Option<Result<(u16, i32), u16>> {
        let password = password.trim().to_uppercase();
        let (word, code) = match password.split_once('-') {
            Some((w, c)) => (w.to_string(), Some(c.replace('Z', "0").replace('Y', "O"))),
            None => (password.clone(), None),
        };
        let index = self.words.iter().position(|w| *w == word)?;
        let level = index as u16 + 2;
        let Some(code) = code else {
            return Some(Ok((level, 0)));
        };
        let parsed = || {
            Some((
                parse_radix(code.get(..5)?, 16)?,
                parse_radix(code.get(5..)?, 34)?,
            ))
        };
        let checked =
            parsed().filter(|&(score, sum)| code.len() > 5 && score * weight(&word) == sum);
        Some(match checked {
            Some((score, _)) => Ok((level, score as i32)),
            None => Err(level),
        })
    }
}
