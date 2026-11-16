//! The standard input of mode b, "send with an instruction" (ADR-0035, section 4): the instruction and the
//! material (the target text) joined into one text, so that both go on the standard input and neither goes into an
//! argument or an environment variable.
//!
//! ```text
//! <instruction>
//!
//! ---
//! <material>
//! ```
//!
//! The instruction is trimmed and its line breaks are LF (it may have several lines); an empty one is refused.
//! MATSU adds nothing else (no tags, no language). The material is not touched but for its line breaks, which
//! become LF as the one-liners' input does (ADR-0029). The material can be large and is read by range from a
//! document, so [`instruction_head`] gives only the part before it; [`compose`] joins both for the callers that
//! have the material in hand.

use crate::oneliner::LfNormalizer;

/// The separator line between the instruction and the material.
const SEPARATOR: &str = "---";

/// The instruction is empty (only white space), so there is nothing to send with the material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmptyInstruction;

impl std::fmt::Display for EmptyInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the instruction is empty")
    }
}

impl std::error::Error for EmptyInstruction {}

/// The instruction as it is sent and recorded: trimmed, with its line breaks in LF. An empty one (only white space)
/// is refused.
pub fn normalized_instruction(instruction: &str) -> Result<String, EmptyInstruction> {
    let instruction = instruction.trim();
    if instruction.is_empty() {
        return Err(EmptyInstruction);
    }
    let mut normalizer = LfNormalizer::new();
    let mut text = normalizer.push(instruction);
    text.push_str(&normalizer.finish());
    Ok(text)
}

/// Everything of the input that comes before the material: the trimmed instruction in LF, a blank line, `---` and
/// the line break after it. The material follows directly.
pub fn instruction_head(instruction: &str) -> Result<String, EmptyInstruction> {
    let mut head = normalized_instruction(instruction)?;
    head.push_str("\n\n");
    head.push_str(SEPARATOR);
    head.push('\n');
    Ok(head)
}

/// The whole input for `instruction` and `material`.
pub fn compose(instruction: &str, material: &str) -> Result<String, EmptyInstruction> {
    let mut input = instruction_head(instruction)?;
    let mut normalizer = LfNormalizer::new();
    input.push_str(&normalizer.push(material));
    input.push_str(&normalizer.finish());
    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_instruction_a_blank_line_a_separator_and_the_material() {
        assert_eq!(
            compose("Translate to English.", "こんにちは\n").unwrap(),
            "Translate to English.\n\n---\nこんにちは\n"
        );
    }

    #[test]
    fn the_normalized_instruction_is_trimmed_and_in_lf() {
        assert_eq!(
            normalized_instruction(" \r\nA\r\nB\rC \n").unwrap(),
            "A\nB\nC"
        );
        assert_eq!(normalized_instruction(" \t\n"), Err(EmptyInstruction));
    }

    #[test]
    fn the_instruction_is_trimmed() {
        assert_eq!(
            compose("  \n\t Fix it. \r\n ", "x").unwrap(),
            "Fix it.\n\n---\nx"
        );
    }

    #[test]
    fn an_empty_instruction_is_refused() {
        for text in ["", "   ", "\r\n\t \n"] {
            assert_eq!(compose(text, "material"), Err(EmptyInstruction), "{text:?}");
            assert_eq!(instruction_head(text), Err(EmptyInstruction), "{text:?}");
        }
    }

    #[test]
    fn a_multi_line_instruction_keeps_its_lines_in_lf() {
        assert_eq!(
            compose("First.\r\nSecond.\rThird.\n\nFourth.", "m\n").unwrap(),
            "First.\nSecond.\nThird.\n\nFourth.\n\n---\nm\n"
        );
    }

    #[test]
    fn the_material_has_its_line_breaks_made_lf_and_nothing_else_changed() {
        assert_eq!(
            compose("Do.", "a\r\nb\rc\n  d  \r\n").unwrap(),
            "Do.\n\n---\na\nb\nc\n  d  \n"
        );
        // a CR at the very end is a line break; no line break is added to a material that has none
        assert_eq!(compose("Do.", "a\r").unwrap(), "Do.\n\n---\na\n");
        assert_eq!(compose("Do.", "a").unwrap(), "Do.\n\n---\na");
        assert_eq!(compose("Do.", "").unwrap(), "Do.\n\n---\n");
    }

    #[test]
    fn a_separator_in_the_material_is_not_special() {
        assert_eq!(compose("Do.", "---\nx\n").unwrap(), "Do.\n\n---\n---\nx\n");
    }

    #[test]
    fn the_head_is_what_comes_before_the_material() {
        assert_eq!(instruction_head("Do.").unwrap(), "Do.\n\n---\n");
    }
}
