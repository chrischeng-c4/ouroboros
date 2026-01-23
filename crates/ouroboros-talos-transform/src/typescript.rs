use anyhow::Result;
use tree_sitter::{Parser, Node};
use crate::{TransformOptions, TransformResult};

/// Transform TypeScript to JavaScript by removing type annotations
pub fn transform_typescript(source: &str, options: &TransformOptions) -> Result<TransformResult> {
    tracing::debug!("Transforming TypeScript (target: {:?})", options.ts_target);

    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())?;

    let tree = parser.parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse TypeScript"))?;

    let root = tree.root_node();

    // Remove type annotations
    let transformed = remove_types(source, &root)?;

    Ok(TransformResult {
        code: transformed,
        source_map: if options.source_maps {
            Some(generate_source_map())
        } else {
            None
        },
    })
}

/// Remove type annotations from TypeScript code
fn remove_types(source: &str, node: &Node) -> Result<String> {
    let mut result = String::new();
    let mut last_pos = 0;
    let mut cursor = node.walk();

    visit_node(source, node, &mut last_pos, &mut result, &mut cursor)?;

    // Append remaining source
    if last_pos < source.len() {
        result.push_str(&source[last_pos..]);
    }

    Ok(result)
}

/// Visit AST node and remove type-related nodes
fn visit_node<'a>(
    source: &str,
    node: &Node<'a>,
    last_pos: &mut usize,
    result: &mut String,
    cursor: &mut tree_sitter::TreeCursor<'a>,
) -> Result<()> {
    for child in node.children(cursor) {
        match child.kind() {
            // Skip type annotations
            "type_annotation" |
            "type_arguments" |
            "type_parameters" |
            "interface_declaration" |
            "type_alias_declaration" |
            "enum_declaration" |
            "as_expression" => {
                // Append source up to this node
                if *last_pos < child.start_byte() {
                    result.push_str(&source[*last_pos..child.start_byte()]);
                }
                // Skip the type annotation entirely
                *last_pos = child.end_byte();
            }

            // Handle optional parameters: foo?: string -> foo
            "optional_parameter" => {
                let param_text = &source[child.byte_range()];
                // Remove the '?' and type annotation
                if let Some(question_pos) = param_text.find('?') {
                    result.push_str(&source[*last_pos..child.start_byte()]);
                    result.push_str(&param_text[..question_pos].trim());
                    *last_pos = child.end_byte();
                }
            }

            // Recursively process children
            _ => {
                if child.child_count() > 0 {
                    let mut child_cursor = child.walk();
                    visit_node(source, &child, last_pos, result, &mut child_cursor)?;
                }
            }
        }
    }

    Ok(())
}

/// Generate source map
fn generate_source_map() -> String {
    r#"{"version":3,"sources":[],"names":[],"mappings":""}"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typescript_basic() {
        let source = "const x: number = 42;";
        let options = TransformOptions::default();
        let result = transform_typescript(source, &options).unwrap();

        // Should not contain type annotation
        assert!(!result.code.contains(": number"));
        assert!(result.code.contains("const x"));
        assert!(result.code.contains("= 42"));
    }

    #[test]
    fn test_typescript_function() {
        let source = "function add(a: number, b: number): number { return a + b; }";
        let options = TransformOptions::default();
        let result = transform_typescript(source, &options).unwrap();

        // Should not contain type annotations
        assert!(!result.code.contains(": number"));
        assert!(result.code.contains("function add"));
        assert!(result.code.contains("return a + b"));
    }

    #[test]
    fn test_typescript_interface() {
        let source = r#"
interface User {
    name: string;
    age: number;
}

const user: User = { name: "Alice", age: 30 };
        "#;
        let options = TransformOptions::default();
        let result = transform_typescript(source, &options).unwrap();

        // Interface should be removed
        assert!(!result.code.contains("interface"));
        // Variable should remain
        assert!(result.code.contains("const user"));
    }
}
