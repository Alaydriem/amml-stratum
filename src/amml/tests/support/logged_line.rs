// One line of the fake loader's mod log, `<level> <message> key=value ...`, split for the asserts.
#[derive(Clone, Debug)]
pub struct LoggedLine {
    pub level: String,
    // the message and the fields, as the line holds them
    pub fields_json: String,
}

impl LoggedLine {
    pub fn new(line: String) -> Self {
        let (level, rest) = line.split_once(' ').unwrap_or((line.as_str(), ""));
        Self { level: level.to_string(), fields_json: rest.to_string() }
    }
}
