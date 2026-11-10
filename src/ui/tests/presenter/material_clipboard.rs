use amml_i18n::localize::Localizer;
use amml_i18n::text::Text;
use stratum_ui::presenter::{MaterialClipboard, MaterialRows};

use crate::support::SnapshotFixture;

// Renders rows in English, the way the copy button does through the context's localizer.
struct English;

impl English {
    fn render(filter: &str, heading: &str) -> String {
        let english = Localizer::english();
        let rows = MaterialRows::build(&SnapshotFixture::list(), filter);
        MaterialClipboard::render(&rows, heading, |text: &Text| text.resolve(&english).into_owned())
    }
}

#[test]
fn clipboard_has_heading_and_one_line_per_row() {
    let text = English::render("", "Willow Manor");
    let lines: Vec<&str> = text.split("\r\n").collect();
    assert_eq!(
        lines,
        [
            "Willow Manor",
            "Stone Bricks: 3,412 (53 stacks + 20)",
            "Spruce Planks: 2,208 (34 stacks + 32)",
            "Deepslate Tiles: 1,536 (24 stacks)",
            "Lantern: 42 (42)",
            ""
        ]
    );
}

#[test]
fn clipboard_uses_crlf() {
    let text = English::render("", "Willow Manor");
    assert!(!text.replace("\r\n", "").contains('\n'), "a bare line feed in {text:?}");
    assert!(text.ends_with("\r\n"));
}

#[test]
fn clipboard_respects_the_filter() {
    let text = English::render("spruce", "Main hall");
    assert_eq!(text, "Main hall\r\nSpruce Planks: 2,208 (34 stacks + 32)\r\n");
}

#[test]
fn clipboard_size_limit_is_one_mebibyte() {
    assert!(MaterialClipboard::fits(&"a".repeat(1_048_576)));
    assert!(!MaterialClipboard::fits(&"a".repeat(1_048_577)));
}
