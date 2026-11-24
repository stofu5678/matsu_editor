//! `Jump(line, option)` of a script macro sets the unit of the go-to-line panel as a key macro's
//! does (`CMacro.cpp` 719-741). The script reaches the core command through the bridge.

use gpui::TestAppContext;
use matsu_plugin::session::Language;

use super::{Fixture, available, setup, write};
use crate::saved_state::other::{JumpUnit, jump_unit};

fn unit(f: &Fixture, cx: &mut gpui::VisualTestContext) -> JumpUnit {
    f.view.read_with(cx, |_, cx| jump_unit(cx))
}

/// Runs the script and checks that the unit it left is `expected` (the start is 改行単位).
fn runs(cx: &mut TestAppContext, language: Language, name: &str, code: &str, expected: JumpUnit) {
    if !available(language) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), name, code);
    let (f, cx) = setup(cx, "a\nb\nc");
    assert_eq!(unit(&f, cx), JumpUnit::Lines);
    f.run_script(cx, &path, |_| {});
    assert_eq!(unit(&f, cx), expected, "{}", f.output(cx));
}

#[gpui::test]
fn a_javascript_jump_with_option_0_sets_the_unit_to_rows(cx: &mut TestAppContext) {
    runs(
        cx,
        Language::JavaScript,
        "j0.cjs",
        "Editor.Jump(2, 0);\n",
        JumpUnit::Rows,
    );
}

#[gpui::test]
fn a_python_jump_with_option_0_sets_the_unit_to_rows(cx: &mut TestAppContext) {
    runs(
        cx,
        Language::Python,
        "j0.py",
        "Editor.Jump(2, 0)\n",
        JumpUnit::Rows,
    );
}
