#[derive(Debug, Clone)]
pub struct FrontMatter {
    pub header: &'static str,
    pub footer: &'static str,
}

pub const DASH: FrontMatter = FrontMatter {
    header: "---\n",
    footer: "\n---\n",
};

pub const EQUAL: FrontMatter = FrontMatter {
    header: "===\n",
    footer: "\n===\n",
};

pub const PLUS: FrontMatter = FrontMatter {
    header: "+++\n",
    footer: "\n+++\n",
};

pub const LUA: FrontMatter = FrontMatter {
    header: "--[===[\n",
    footer: "\n]===]\n",
};

impl FrontMatter {
    /// Parse front matter if present
    ///
    /// Returns a tuple:
    /// - `Some(&str)` containing the front matter (excluding delimiters),
    /// - `&str` containing the rest of the body.
    pub fn parse<'a>(&self, input: &'a str) -> (Option<&'a str>, &'a str) {
        // Strip BOM if present.
        let input = strip_bom(input);

        // Must start with header (front matter opening).
        if !input.starts_with(self.header) {
            return (None, input);
        }

        // Position just after the opening delimiter.
        let start = self.header.len();

        // Look for the closing delimiter.
        let rest = &input[start..];
        if let Some(end) = rest.find(self.footer) {
            let front_matter = &rest[..end];
            let body_start = start + end + self.footer.len();
            let body = &input[body_start..];
            (Some(front_matter), body)
        } else {
            // No closing delimiter found.
            (None, input)
        }
    }

    /// Parse front matter if present
    ///
    /// Returns the front matter exluding delimiters if present and alters the
    /// given string to be the rest of the body.
    pub fn parse_in_place(&self, input: &mut String) -> Option<String> {
        // Strip BOM if present.
        let mut start = bom_len(input);

        // Must start with header (front matter opening).
        if !input[start..].starts_with(self.header) {
            return None;
        }

        // Position just after the opening delimiter.
        start += self.header.len();

        // Look for the closing delimiter.
        if let Some(end) = input[start..].find(self.footer) {
            let front_matter = input[start..end + start].to_string();
            // Drain once. Each drain is an memmove.
            let _ = input.drain(0..start + end + self.footer.len());
            Some(front_matter)
        } else {
            None
        }
    }
}

// Strip BOM if present.
fn strip_bom(s: &str) -> &str {
    s.strip_prefix("\u{FEFF}").unwrap_or(s)
}

// YOUNG ME: Ooo, this is expressive.
// fn strip_bom_inplace(s: &mut String) {
//     const BOM: &str = "\u{FEFF}";
//     if s.starts_with(BOM) {
//         // OLD ME: Oof, we're gonna memmove the whole string for this?
//         s.drain(..BOM.len());
//     }
// }

// OLD ME: But this is performant.
fn bom_len(s: &str) -> usize {
    const BOM: &str = "\u{FEFF}";
    if s.starts_with(BOM) { BOM.len() } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> (Option<&str>, &str) {
        EQUAL.parse(input)
    }

    fn parse_in_place(input: &mut String) -> Option<String> {
        EQUAL.parse_in_place(input)
    }

    #[test]
    fn test_with_front_matter() {
        let input = "\
===
title: Test
date: 2025-05-27
===
# Content
This is the body.";
        let (fm, body) = parse(input);

        assert_eq!(fm, Some("title: Test\ndate: 2025-05-27"));
        assert_eq!(body, "# Content\nThis is the body.");
    }

    #[test]
    fn test_with_lua_front_matter() {
        let input = "\
--[===[
title: Test
date: 2025-05-27
]===]
# Content
This is the body.";
        let (fm, body) = LUA.parse(input);

        assert_eq!(fm, Some("title: Test\ndate: 2025-05-27"));
        assert_eq!(body, "# Content\nThis is the body.");
    }

    #[test]
    fn test_without_front_matter() {
        let input = "# Content\nNo front matter here.";
        let (fm, body) = parse(input);

        assert!(fm.is_none());
        assert_eq!(body, input);
    }

    #[test]
    fn test_front_matter_without_end() {
        let input = "\
===
title: Test
Still in front matter
No closing delimiter";
        let (fm, body) = parse(input);

        assert!(fm.is_none());
        assert_eq!(body, input);
    }

    #[test]
    fn test_empty_input() {
        let input = "";
        let (fm, body) = parse(input);

        assert!(fm.is_none());
        assert_eq!(body, "");
    }

    #[test]
    fn test_only_front_matter() {
        let input = "\
===
foo: bar
===
";
        let (fm, body) = parse(input);

        assert_eq!(fm, Some("foo: bar"));
        assert_eq!(body, "");
    }

    mod in_place {
        use super::*;

        #[test]
        fn test_with_front_matter() {
            let mut input = "\
===\ntitle: Test\ndate: 2025-05-27\n===\n# Content\nBody text."
                .to_string();

            let fm = parse_in_place(&mut input);

            assert_eq!(fm, Some("title: Test\ndate: 2025-05-27".to_string()));
            assert_eq!(input, "# Content\nBody text.");
        }

        #[test]
        fn test_without_front_matter() {
            let original = "# Content\nNo front matter here.";
            let mut input = original.to_string();

            let fm = parse_in_place(&mut input);

            assert_eq!(fm, None);
            assert_eq!(input, original);
        }

        #[test]
        fn test_front_matter_without_end() {
            let original = "\
===\ntitle: Incomplete\nno end marker";
            let mut input = original.to_string();
            assert_eq!(original, "===\ntitle: Incomplete\nno end marker");

            let fm = parse_in_place(&mut input);

            assert_eq!(fm, None);
            assert_eq!(input, original);
        }

        #[test]
        fn test_empty_input() {
            let mut input = "".to_string();

            let fm = parse_in_place(&mut input);

            assert_eq!(fm, None);
            assert_eq!(input, "");
        }

        #[test]
        fn test_only_front_matter() {
            let mut input = "===\nfoo: bar\n===\n".to_string();

            let fm = parse_in_place(&mut input);

            assert_eq!(fm, Some("foo: bar".to_string()));
            assert_eq!(input, "");
        }

        #[test]
        fn test_trailing_newlines_after_front_matter() {
            let mut input = "===\nfoo: bar\n===\n\n\nHello".to_string();

            let fm = parse_in_place(&mut input);

            assert_eq!(fm, Some("foo: bar".to_string()));
        }
    }
}
