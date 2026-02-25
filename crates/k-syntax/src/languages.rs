use std::path::Path;

pub struct LanguageDetector;

impl LanguageDetector {
    pub fn detect_from_path(path: &Path) -> Option<String> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| Self::extension_to_language(ext))
            .map(String::from)
    }

    pub fn extension_to_language(ext: &str) -> Option<&'static str> {
        match ext.to_lowercase().as_str() {
            "rs" => Some("Rust"),
            "py" | "pyw" => Some("Python"),
            "js" | "mjs" | "cjs" => Some("JavaScript"),
            "ts" | "mts" => Some("TypeScript"),
            "tsx" => Some("TypeScript (JSX)"),
            "jsx" => Some("JavaScript (JSX)"),
            "go" => Some("Go"),
            "rb" => Some("Ruby"),
            "java" => Some("Java"),
            "c" | "h" => Some("C"),
            "cpp" | "cxx" | "cc" | "hpp" | "hxx" => Some("C++"),
            "cs" => Some("C#"),
            "swift" => Some("Swift"),
            "kt" | "kts" => Some("Kotlin"),
            "php" => Some("PHP"),
            "html" | "htm" => Some("HTML"),
            "css" => Some("CSS"),
            "scss" | "sass" => Some("SCSS"),
            "json" => Some("JSON"),
            "yaml" | "yml" => Some("YAML"),
            "toml" => Some("TOML"),
            "xml" => Some("XML"),
            "md" | "markdown" => Some("Markdown"),
            "sql" => Some("SQL"),
            "sh" | "bash" | "zsh" => Some("Shell"),
            "lua" => Some("Lua"),
            "vim" => Some("VimL"),
            "zig" => Some("Zig"),
            "ex" | "exs" => Some("Elixir"),
            "erl" | "hrl" => Some("Erlang"),
            "hs" => Some("Haskell"),
            "ml" | "mli" => Some("OCaml"),
            "r" => Some("R"),
            "dart" => Some("Dart"),
            "dockerfile" => Some("Dockerfile"),
            "makefile" | "mk" => Some("Makefile"),
            "tf" => Some("Terraform"),
            "proto" => Some("Protocol Buffers"),
            _ => None,
        }
    }

    pub fn extension_to_syntect_name(ext: &str) -> Option<&'static str> {
        match ext.to_lowercase().as_str() {
            "rs" => Some("Rust"),
            "py" | "pyw" => Some("Python"),
            "js" | "mjs" | "cjs" => Some("JavaScript"),
            "ts" | "mts" => Some("TypeScript"),
            "tsx" => Some("TypeScript"),
            "jsx" => Some("JavaScript"),
            "go" => Some("Go"),
            "rb" => Some("Ruby"),
            "java" => Some("Java"),
            "c" | "h" => Some("C"),
            "cpp" | "cxx" | "cc" | "hpp" | "hxx" => Some("C++"),
            "cs" => Some("C#"),
            "swift" => Some("Swift"),
            "kt" | "kts" => Some("Kotlin"),
            "php" => Some("PHP"),
            "html" | "htm" => Some("HTML"),
            "css" => Some("CSS"),
            "scss" | "sass" => Some("CSS"),
            "json" => Some("JSON"),
            "yaml" | "yml" => Some("YAML"),
            "toml" => Some("TOML"),
            "xml" => Some("XML"),
            "md" | "markdown" => Some("Markdown"),
            "sql" => Some("SQL"),
            "sh" | "bash" | "zsh" => Some("Bourne Again Shell (bash)"),
            _ => None,
        }
    }
}
