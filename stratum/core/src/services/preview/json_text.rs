// The game's JSON files carry `//` line comments, which strict JSON does not allow.
pub struct JsonText;

impl JsonText {
    // The text with every `//` comment outside a string removed.
    pub fn strip_comments(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        let mut in_string = false;
        let mut escaped = false;
        while let Some(c) = chars.next() {
            if in_string {
                out.push(c);
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    in_string = false;
                }
                continue;
            }
            if c == '/' && chars.peek() == Some(&'/') {
                for skipped in chars.by_ref() {
                    if skipped == '\n' {
                        out.push('\n');
                        break;
                    }
                }
                continue;
            }
            if c == '"' {
                in_string = true;
            }
            out.push(c);
        }
        out
    }
}
