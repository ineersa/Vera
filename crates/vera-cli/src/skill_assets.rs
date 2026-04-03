//! Embedded skill files that `vera agent install` writes to agent skill dirs.

pub struct SkillFile {
    pub relative_path: &'static str,
    pub contents: &'static str,
}

pub const VERA_SKILL_NAME: &str = "vera";

pub const VERA_SKILL_FILES: &[SkillFile] = &[SkillFile {
    relative_path: "SKILL.md",
    contents: include_str!("../../../skills/vera/SKILL.md"),
}];
