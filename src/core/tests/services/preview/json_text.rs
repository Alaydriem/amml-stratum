use stratum_core::services::preview::JsonText;

#[test]
fn line_comments_go_and_slashes_in_strings_stay() {
    let text = "{\n  \"a\": { // a note\n    \"url\": \"http://x\\\"//y\"\n  }\n}";
    let stripped = JsonText::strip_comments(text);
    let value: serde_json::Value = serde_json::from_str(&stripped).unwrap();
    assert_eq!(value["a"]["url"], "http://x\"//y");
}
