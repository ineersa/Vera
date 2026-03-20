//! Shared types used across Vera's core modules.

use serde::{Deserialize, Serialize};

/// A chunk of source code extracted from a parsed file.
///
/// This is the fundamental unit that gets indexed, embedded, and retrieved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Unique identifier for this chunk.
    pub id: String,
    /// Repository-relative file path.
    pub file_path: String,
    /// 1-based start line in the source file.
    pub line_start: u32,
    /// 1-based end line in the source file (inclusive).
    pub line_end: u32,
    /// The actual source code content of this chunk.
    pub content: String,
    /// Detected programming language.
    pub language: Language,
    /// Type of symbol this chunk represents (if any).
    pub symbol_type: Option<SymbolType>,
    /// Name of the symbol (if applicable).
    pub symbol_name: Option<String>,
}

/// Programming language of a source file or chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    C,
    Cpp,
    Ruby,
    Swift,
    Kotlin,
    Scala,
    Zig,
    Lua,
    Bash,
    /// Structural / config formats (Tier 1B).
    Toml,
    Yaml,
    Json,
    Markdown,
    /// Fallback for unrecognized file types (Tier 0).
    Unknown,
}

impl Language {
    /// Detect language from a file extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Self::Rust,
            "ts" | "tsx" => Self::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Self::JavaScript,
            "py" | "pyi" => Self::Python,
            "go" => Self::Go,
            "java" => Self::Java,
            "c" | "h" => Self::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => Self::Cpp,
            "rb" => Self::Ruby,
            "swift" => Self::Swift,
            "kt" | "kts" => Self::Kotlin,
            "scala" | "sc" => Self::Scala,
            "zig" => Self::Zig,
            "lua" => Self::Lua,
            "sh" | "bash" | "zsh" => Self::Bash,
            "toml" => Self::Toml,
            "yaml" | "yml" => Self::Yaml,
            "json" => Self::Json,
            "md" | "markdown" => Self::Markdown,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Python => "python",
            Self::Go => "go",
            Self::Java => "java",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Ruby => "ruby",
            Self::Swift => "swift",
            Self::Kotlin => "kotlin",
            Self::Scala => "scala",
            Self::Zig => "zig",
            Self::Lua => "lua",
            Self::Bash => "bash",
            Self::Toml => "toml",
            Self::Yaml => "yaml",
            Self::Json => "json",
            Self::Markdown => "markdown",
            Self::Unknown => "unknown",
        };
        write!(f, "{name}")
    }
}

/// Type of symbol extracted from source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolType {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Trait,
    Interface,
    TypeAlias,
    Constant,
    Variable,
    Module,
    /// A fallback chunk not aligned to a specific symbol.
    Block,
}

impl std::fmt::Display for SymbolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Class => "class",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Interface => "interface",
            Self::TypeAlias => "type_alias",
            Self::Constant => "constant",
            Self::Variable => "variable",
            Self::Module => "module",
            Self::Block => "block",
        };
        write!(f, "{name}")
    }
}

/// A search result returned by the retrieval pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Repository-relative file path.
    pub file_path: String,
    /// 1-based start line.
    pub line_start: u32,
    /// 1-based end line (inclusive).
    pub line_end: u32,
    /// The code content of this result.
    pub content: String,
    /// Programming language.
    pub language: Language,
    /// Relevance score (higher is better).
    pub score: f64,
    /// Symbol name, if the result corresponds to a named symbol.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_name: Option<String>,
    /// Symbol type, if the result corresponds to a typed symbol.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_type: Option<SymbolType>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_from_extension_rust() {
        assert_eq!(Language::from_extension("rs"), Language::Rust);
    }

    #[test]
    fn language_from_extension_typescript() {
        assert_eq!(Language::from_extension("ts"), Language::TypeScript);
        assert_eq!(Language::from_extension("tsx"), Language::TypeScript);
    }

    #[test]
    fn language_from_extension_python() {
        assert_eq!(Language::from_extension("py"), Language::Python);
        assert_eq!(Language::from_extension("pyi"), Language::Python);
    }

    #[test]
    fn language_from_extension_unknown() {
        assert_eq!(Language::from_extension("xyz"), Language::Unknown);
    }

    #[test]
    fn language_from_extension_case_insensitive() {
        assert_eq!(Language::from_extension("RS"), Language::Rust);
        assert_eq!(Language::from_extension("Py"), Language::Python);
    }

    #[test]
    fn language_display() {
        assert_eq!(Language::Rust.to_string(), "rust");
        assert_eq!(Language::TypeScript.to_string(), "typescript");
        assert_eq!(Language::Unknown.to_string(), "unknown");
    }

    #[test]
    fn symbol_type_display() {
        assert_eq!(SymbolType::Function.to_string(), "function");
        assert_eq!(SymbolType::Class.to_string(), "class");
        assert_eq!(SymbolType::Block.to_string(), "block");
    }

    #[test]
    fn chunk_serialization_round_trip() {
        let chunk = Chunk {
            id: "test-1".to_string(),
            file_path: "src/main.rs".to_string(),
            line_start: 1,
            line_end: 10,
            content: "fn main() {}".to_string(),
            language: Language::Rust,
            symbol_type: Some(SymbolType::Function),
            symbol_name: Some("main".to_string()),
        };
        let json = serde_json::to_string(&chunk).unwrap();
        let deserialized: Chunk = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test-1");
        assert_eq!(deserialized.file_path, "src/main.rs");
        assert_eq!(deserialized.language, Language::Rust);
        assert_eq!(deserialized.symbol_name, Some("main".to_string()));
    }

    #[test]
    fn search_result_serialization_omits_none() {
        let result = SearchResult {
            file_path: "lib.rs".to_string(),
            line_start: 5,
            line_end: 20,
            content: "pub fn example() {}".to_string(),
            language: Language::Rust,
            score: 0.95,
            symbol_name: None,
            symbol_type: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.contains("symbol_name"));
        assert!(!json.contains("symbol_type"));
    }
}
