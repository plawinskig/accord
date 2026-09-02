pub const APP_QUALIFIER: &str = "com";
pub const APP_ORG: &str = "greggy";
pub const APP_NAME: &str = "accord";

pub const DB_FILE_NAME: &str = "accord.sqlite";
pub const ATTACHMENTS_DIR: &str = "attachments";

// The character (or string) that triggers #tag-style auto-detection in note
// content. Change this one constant to switch the whole app to a different
// trigger - e.g. "~" or "::" - nothing else needs to be touched, since
// tags::tag_token_regex() builds its pattern from this value (escaped, so
// even a regex-special character like "$" or "." would be safe to use here).
pub const TAG_TRIGGER: &str = "::";
