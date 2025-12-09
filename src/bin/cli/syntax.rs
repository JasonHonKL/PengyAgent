use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use std::sync::OnceLock;
use tree_sitter::{Language, Parser, Query, QueryCursor};

// Language grammars - lazy loaded using OnceLock for thread safety
static RUST_LANG: OnceLock<Language> = OnceLock::new();
static PYTHON_LANG: OnceLock<Language> = OnceLock::new();
static JAVASCRIPT_LANG: OnceLock<Language> = OnceLock::new();
static TYPESCRIPT_LANG: OnceLock<Language> = OnceLock::new();
static GO_LANG: OnceLock<Language> = OnceLock::new();
static JAVA_LANG: OnceLock<Language> = OnceLock::new();
static C_LANG: OnceLock<Language> = OnceLock::new();
static CPP_LANG: OnceLock<Language> = OnceLock::new();

fn get_language(lang: &str) -> Option<&'static Language> {
    Some(match lang.to_lowercase().as_str() {
        "rust" | "rs" => RUST_LANG.get_or_init(|| tree_sitter_rust::language()),
        "python" | "py" => PYTHON_LANG.get_or_init(|| tree_sitter_python::language()),
        "javascript" | "js" => JAVASCRIPT_LANG.get_or_init(|| tree_sitter_javascript::language()),
        "typescript" | "ts" | "tsx" => TYPESCRIPT_LANG.get_or_init(|| tree_sitter_typescript::language_typescript()),
        "go" => GO_LANG.get_or_init(|| tree_sitter_go::language()),
        "java" => JAVA_LANG.get_or_init(|| tree_sitter_java::language()),
        "c" => C_LANG.get_or_init(|| tree_sitter_c::language()),
        "cpp" | "c++" | "cxx" | "cc" => CPP_LANG.get_or_init(|| tree_sitter_cpp::language()),
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind {
    Comment,
    Keyword,
    String,
    Number,
    Function,
    Variable,
    Type,
    Operator,
    Punctuation,
    Constant,
    Attribute,
    Normal,
}

fn map_capture_to_kind(capture_name: &str) -> TokenKind {
    match capture_name {
        "comment" => TokenKind::Comment,
        "keyword" => TokenKind::Keyword,
        "string" => TokenKind::String,
        "number" => TokenKind::Number,
        "function" | "function.builtin" | "method" => TokenKind::Function,
        "variable" | "variable.parameter" => TokenKind::Variable,
        "type" | "type.builtin" => TokenKind::Type,
        "operator" => TokenKind::Operator,
        "constant" | "constant.builtin" => TokenKind::Constant,
        "attribute" => TokenKind::Attribute,
        _ => TokenKind::Normal,
    }
}

fn get_style_for_kind(kind: TokenKind, accent: Color, bg: Color) -> Style {
    let base = Style::default().bg(bg);
    match kind {
        TokenKind::Comment => base.fg(Color::Rgb(100, 100, 120)),
        TokenKind::Keyword => base.fg(accent).add_modifier(Modifier::BOLD),
        TokenKind::String => base.fg(Color::Rgb(150, 200, 150)),
        TokenKind::Number => base.fg(Color::Rgb(180, 200, 255)),
        TokenKind::Function => base.fg(Color::Rgb(200, 180, 100)),
        TokenKind::Variable => base.fg(Color::White),
        TokenKind::Type => base.fg(Color::Rgb(100, 200, 255)),
        TokenKind::Operator => base.fg(Color::Rgb(200, 150, 200)),
        TokenKind::Punctuation => base.fg(Color::Rgb(150, 150, 150)),
        TokenKind::Constant => base.fg(Color::Rgb(255, 200, 100)),
        TokenKind::Attribute => base.fg(Color::Rgb(200, 200, 100)),
        TokenKind::Normal => base.fg(Color::White),
    }
}

pub fn highlight_line_with_tree_sitter(
    line: &str,
    lang: &str,
    accent: Color,
    bg: Color,
) -> Vec<Span<'static>> {
    let language = match get_language(lang) {
        Some(lang) => lang,
        None => {
            // Fallback to plain text
            return vec![Span::styled(line.to_string(), Style::default().bg(bg).fg(Color::White))];
        }
    };

    let mut parser = Parser::new();
    // Language needs to be cloned/dereferenced for tree-sitter 0.20
    let lang_clone = unsafe { std::ptr::read(language as *const Language) };
    if parser.set_language(lang_clone).is_err() {
        // Fallback to basic highlighting if language setup fails
        return apply_basic_highlighting(line, lang, accent, bg);
    }

    // Parse the line - use None for old_tree since we're parsing individual lines
    let tree = match parser.parse(line, None) {
        Some(t) => t,
        None => {
            // Fallback to basic highlighting if parsing fails
            return apply_basic_highlighting(line, lang, accent, bg);
        }
    };
    let root_node = tree.root_node();
    
    // If the tree has errors, still try to highlight what we can
    if root_node.has_error() && root_node.child_count() == 0 {
        // If parsing completely failed, fallback to basic highlighting
        return apply_basic_highlighting(line, lang, accent, bg);
    }

    // Build query patterns for common syntax elements
    let query_patterns = match lang.to_lowercase().as_str() {
        "rust" | "rs" => r#"
            (line_comment) @comment
            (block_comment) @comment
            (string_literal) @string
            (char_literal) @string
            (integer_literal) @number
            (float_literal) @number
            [
                "fn" "let" "mut" "pub" "struct" "impl" "trait" "enum" "match" "use" "mod"
                "ref" "if" "else" "loop" "for" "while" "in" "move" "return" "async" "await"
                "const" "static" "type" "where" "unsafe" "extern" "crate" "self" "Self"
            ] @keyword
            (function_item name: (identifier) @function)
            (type_identifier) @type
            (primitive_type) @type
        "#,
        "python" | "py" => r#"
            (comment) @comment
            (string) @string
            (integer) @number
            (float) @number
            [
                "def" "class" "import" "from" "return" "if" "elif" "else" "for" "while" "in"
                "with" "as" "lambda" "yield" "async" "await" "try" "except" "finally" "raise"
                "pass" "break" "continue" "and" "or" "not" "is" "None" "True" "False"
            ] @keyword
            (function_definition name: (identifier) @function)
            (class_definition name: (identifier) @type)
        "#,
        "go" => r#"
            (comment) @comment
            (interpreted_string_literal) @string
            (raw_string_literal) @string
            (int_literal) @number
            (float_literal) @number
            [
                "func" "var" "const" "type" "struct" "interface" "if" "else" "for" "range"
                "switch" "case" "default" "return" "go" "defer" "select" "chan" "map"
                "package" "import" "break" "continue" "fallthrough" "goto"
            ] @keyword
            (function_declaration name: (identifier) @function)
            (type_identifier) @type
        "#,
        "javascript" | "js" | "typescript" | "ts" | "tsx" => r#"
            (comment) @comment
            (string) @string
            (number) @number
            [
                "function" "const" "let" "var" "import" "from" "export" "return" "if" "else"
                "for" "while" "async" "await" "class" "extends" "new" "this" "super"
                "try" "catch" "finally" "throw" "typeof" "instanceof" "in" "of"
            ] @keyword
            (function_declaration name: (identifier) @function)
            (method_definition name: (property_identifier) @function)
            (class_declaration name: (type_identifier) @type)
        "#,
        "java" => r#"
            (line_comment) @comment
            (block_comment) @comment
            (string_literal) @string
            (character_literal) @string
            (decimal_integer_literal) @number
            (hex_integer_literal) @number
            (octal_integer_literal) @number
            (binary_integer_literal) @number
            (floating_point_literal) @number
            [
                "abstract" "assert" "break" "case" "catch" "class" "const" "continue"
                "default" "do" "else" "enum" "extends" "final" "finally" "for" "if"
                "implements" "import" "instanceof" "interface" "native" "new" "package"
                "private" "protected" "public" "return" "static" "strictfp" "super"
                "switch" "synchronized" "this" "throw" "throws" "transient" "try"
                "void" "volatile" "while" "true" "false" "null"
            ] @keyword
            (method_declaration name: (identifier) @function)
            (class_declaration name: (identifier) @type)
            (interface_declaration name: (identifier) @type)
            (type_identifier) @type
            (primitive_type) @type
        "#,
        _ => r#"
            (comment) @comment
            (string) @string
            (number) @number
        "#,
    };

    let lang_clone_for_query = unsafe { std::ptr::read(language as *const Language) };
    let query = match Query::new(lang_clone_for_query, query_patterns) {
        Ok(q) => q,
        Err(_) => {
            // Fallback to basic highlighting if query fails
            return apply_basic_highlighting(line, lang, accent, bg);
        }
    };

    let mut cursor = QueryCursor::new();
    let mut highlights: Vec<(usize, usize, TokenKind)> = Vec::new();

    for m in cursor.matches(&query, root_node, line.as_bytes()) {
        for capture in m.captures {
            let node = capture.node;
            let start = node.start_byte();
            let end = node.end_byte();
            let kind = map_capture_to_kind(&query.capture_names()[capture.index as usize]);
            highlights.push((start, end, kind));
        }
    }

    // Sort by start position
    highlights.sort_by_key(|(start, _, _)| *start);

    // Build spans
    let mut spans = Vec::new();
    let mut last_pos = 0;

    for (start, end, kind) in highlights {
        // Ensure valid byte ranges
        let start = start.min(line.len());
        let end = end.min(line.len());
        if start >= end || start < last_pos {
            continue;
        }

        // Add text before this highlight
        if start > last_pos {
            let text = &line[last_pos..start];
            if !text.is_empty() {
                spans.push(Span::styled(
                    text.to_string(),
                    Style::default().bg(bg).fg(Color::White),
                ));
            }
        }

        // Add highlighted text
        let text = &line[start..end];
        if !text.is_empty() {
            let style = get_style_for_kind(kind, accent, bg);
            spans.push(Span::styled(text.to_string(), style));
        }

        last_pos = end;
    }

    // Add remaining text
    if last_pos < line.len() {
        let text = &line[last_pos..];
        if !text.is_empty() {
            spans.push(Span::styled(
                text.to_string(),
                Style::default().bg(bg).fg(Color::White),
            ));
        }
    }

    if spans.is_empty() {
        // Fallback if no spans were created - try basic keyword highlighting
        spans = apply_basic_highlighting(line, lang, accent, bg);
        if spans.is_empty() {
            spans.push(Span::styled(
                line.to_string(),
                Style::default().bg(bg).fg(Color::White),
            ));
        }
    }

    spans
}

// Fallback basic highlighting when tree-sitter fails
fn apply_basic_highlighting(line: &str, lang: &str, accent: Color, bg: Color) -> Vec<Span<'static>> {
    use std::collections::HashSet;
    let mut spans = Vec::new();
    let base_style = Style::default().bg(bg);
    
    // Simple keyword detection for common languages
    let keywords: HashSet<&str> = match lang.to_lowercase().as_str() {
        "java" => {
            vec!["public", "private", "protected", "class", "static", "void", "main", "String", 
                 "System", "out", "println", "int", "boolean", "if", "else", "for", "while", 
                 "return", "new", "this", "super", "extends", "implements", "import", "package"]
                .into_iter().collect()
        },
        "rust" => {
            vec!["fn", "let", "mut", "pub", "struct", "impl", "use", "mod", "trait", "enum",
                 "match", "if", "else", "for", "while", "return", "async", "await"]
                .into_iter().collect()
        },
        "python" => {
            vec!["def", "class", "import", "from", "return", "if", "elif", "else", "for", "while",
                 "in", "with", "as", "lambda", "yield", "async", "await", "try", "except"]
                .into_iter().collect()
        },
        "go" => {
            vec!["func", "package", "import", "var", "const", "type", "struct", "interface",
                 "if", "else", "for", "range", "switch", "case", "return"]
                .into_iter().collect()
        },
        "javascript" | "js" | "typescript" | "ts" => {
            vec!["function", "const", "let", "var", "class", "return", "if", "else", "for",
                 "while", "async", "await", "import", "export", "from", "new", "this"]
                .into_iter().collect()
        },
        _ => HashSet::new(),
    };
    
    if keywords.is_empty() {
        // No keywords for this language, just return plain text
        return vec![Span::styled(line.to_string(), base_style.fg(Color::White))];
    }
    
    // Tokenize the line by splitting on word boundaries
    let mut current_token = String::new();
    let mut in_string = false;
    let mut string_char = '\0';
    let mut in_comment = false;
    
    for (idx, ch) in line.char_indices() {
        if in_comment {
            current_token.push(ch);
            continue;
        }
        
        if in_string {
            current_token.push(ch);
            if ch == string_char && (idx == 0 || line.chars().nth(idx.saturating_sub(1)) != Some('\\')) {
                // End of string
                spans.push(Span::styled(current_token.clone(), base_style.fg(Color::Rgb(150, 200, 150))));
                current_token.clear();
                in_string = false;
            }
            continue;
        }
        
        if ch == '"' || ch == '\'' {
            // Start of string
            if !current_token.is_empty() {
                let token_str = current_token.clone();
                let clean = token_str.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                if keywords.contains(clean) {
                    spans.push(Span::styled(token_str, base_style.fg(accent).add_modifier(Modifier::BOLD)));
                } else if token_str.chars().all(|c| c.is_ascii_digit() || c == '.') {
                    spans.push(Span::styled(token_str, base_style.fg(Color::Rgb(180, 200, 255))));
                } else {
                    spans.push(Span::styled(token_str, base_style.fg(Color::White)));
                }
                current_token.clear();
            }
            current_token.push(ch);
            in_string = true;
            string_char = ch;
        } else if ch.is_alphanumeric() || ch == '_' {
            current_token.push(ch);
        } else {
            // Non-alphanumeric character
            if !current_token.is_empty() {
                let token_str = current_token.clone();
                let clean = token_str.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                if keywords.contains(clean) {
                    spans.push(Span::styled(token_str, base_style.fg(accent).add_modifier(Modifier::BOLD)));
                } else if token_str.chars().all(|c| c.is_ascii_digit() || c == '.') {
                    spans.push(Span::styled(token_str, base_style.fg(Color::Rgb(180, 200, 255))));
                } else {
                    spans.push(Span::styled(token_str, base_style.fg(Color::White)));
                }
                current_token.clear();
            }
            // Add punctuation/whitespace
            spans.push(Span::styled(ch.to_string(), base_style.fg(Color::Rgb(150, 150, 150))));
        }
    }
    
    // Handle remaining token
    if !current_token.is_empty() {
        let token_str = current_token;
        let clean = token_str.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        if keywords.contains(clean) {
            spans.push(Span::styled(token_str, base_style.fg(accent).add_modifier(Modifier::BOLD)));
        } else if token_str.chars().all(|c| c.is_ascii_digit() || c == '.') {
            spans.push(Span::styled(token_str, base_style.fg(Color::Rgb(180, 200, 255))));
        } else {
            spans.push(Span::styled(token_str, base_style.fg(Color::White)));
        }
    }
    
    if spans.is_empty() {
        spans.push(Span::styled(line.to_string(), base_style.fg(Color::White)));
    }
    
    spans
}

