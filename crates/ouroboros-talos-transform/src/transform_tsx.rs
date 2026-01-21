use anyhow::Result;
use tree_sitter::{Parser, Node};
use crate::{TransformOptions, TransformResult};

/// Transform TSX to JavaScript in a single pass
///
/// This module addresses the critical bugs identified in Issue #101:
/// - Bug 1: Uses LANGUAGE_TSX parser (not JavaScript parser)
/// - Bug 2: Single-pass transformation (not dual pipeline)
/// - Bug 3: Proper error handling (no default "div" fallback)
pub fn transform_tsx(source: &str, options: &TransformOptions) -> Result<TransformResult> {
    tracing::debug!("Transforming TSX (single-pass, jsx_automatic: {})", options.jsx_automatic);

    let mut parser = Parser::new();
    // ✅ Fix Bug #2: Use correct parser for TSX files
    parser.set_language(&tree_sitter_typescript::LANGUAGE_TSX.into())?;

    let tree = parser.parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse TSX"))?;

    let root = tree.root_node();

    // ✅ Fix Bug #1: Single-pass transformation (removes types + transforms JSX)
    let mut transformed = transform_node(source, &root, options)?;

    // Add jsx-runtime imports if using automatic runtime
    if options.jsx_automatic && has_jsx(&root) {
        let runtime_import = "import { jsx, jsxs, Fragment } from 'react/jsx-runtime';\n";
        transformed = runtime_import.to_string() + &transformed;
    }

    Ok(TransformResult {
        code: transformed,
        source_map: if options.source_maps {
            Some(generate_source_map(source))
        } else {
            None
        },
    })
}

/// Check if the AST contains JSX elements
fn has_jsx(node: &Node) -> bool {
    if matches!(node.kind(), "jsx_element" | "jsx_self_closing_element" | "jsx_fragment") {
        return true;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if has_jsx(&child) {
            return true;
        }
    }

    false
}

/// Transform a single AST node
///
/// This function simultaneously:
/// 1. Removes TypeScript type annotations
/// 2. Transforms JSX elements to jsx() calls
fn transform_node(source: &str, node: &Node, options: &TransformOptions) -> Result<String> {
    let mut result = String::new();
    let mut cursor = node.walk();
    let mut last_pos = node.start_byte();

    for child in node.children(&mut cursor) {
        // Check if we should skip this node (TypeScript-specific)
        if should_skip_node(&child) {
            // Append source up to this node (but not including the type annotation)
            if last_pos < child.start_byte() {
                let before_type = &source[last_pos..child.start_byte()];
                // Trim trailing whitespace before type annotation (like ": ")
                result.push_str(before_type.trim_end());
            }
            // Skip the type annotation entirely, but add a space after if needed
            last_pos = child.end_byte();
            // Check if there's a non-whitespace character following
            if last_pos < source.len() {
                let next_char = source.as_bytes().get(last_pos).copied();
                // Add space if next char is not whitespace and not punctuation
                if let Some(ch) = next_char {
                    if ch != b' ' && ch != b'\n' && ch != b'\r' && ch != b'\t'
                       && ch != b';' && ch != b',' && ch != b')' && ch != b'}'
                       && ch != b'=' && ch != b'>' {
                        result.push(' ');
                    }
                }
            }
            continue;
        }

        // Preserve whitespace before node
        if child.start_byte() > last_pos {
            result.push_str(&source[last_pos..child.start_byte()]);
        }

        match child.kind() {
            // Handle JSX elements
            "jsx_element" | "jsx_self_closing_element" => {
                result.push_str(&transform_jsx_element(source, &child, options)?);
                last_pos = child.end_byte();
            }
            "jsx_fragment" => {
                result.push_str(&transform_jsx_fragment(source, &child, options)?);
                last_pos = child.end_byte();
            }

            // Handle optional parameters: foo?: string -> foo
            "optional_parameter" => {
                let param_text = &source[child.byte_range()];
                if let Some(question_pos) = param_text.find('?') {
                    result.push_str(&param_text[..question_pos].trim());
                } else {
                    result.push_str(param_text);
                }
                last_pos = child.end_byte();
            }

            // Recursively process other nodes
            _ => {
                if child.child_count() > 0 {
                    result.push_str(&transform_node(source, &child, options)?);
                    last_pos = child.end_byte();
                } else {
                    result.push_str(&source[child.byte_range()]);
                    last_pos = child.end_byte();
                }
            }
        }
    }

    // Append any remaining source
    if last_pos < node.end_byte() {
        result.push_str(&source[last_pos..node.end_byte()]);
    }

    Ok(result)
}

/// Check if a node should be skipped (TypeScript-specific syntax)
fn should_skip_node(node: &Node) -> bool {
    matches!(
        node.kind(),
        "type_annotation"
            | "type_arguments"
            | "type_parameters"
            | "interface_declaration"
            | "type_alias_declaration"
            | "enum_declaration"
            | "as_expression"
    )
}

/// Transform JSX element to jsx() or React.createElement() call
fn transform_jsx_element(source: &str, node: &Node, options: &TransformOptions) -> Result<String> {
    // ✅ Fix Bug #3: Proper error handling instead of defaulting to "div"
    let tag_name = extract_tag_name(source, node)
        .ok_or_else(|| anyhow::anyhow!("Failed to extract JSX tag name from: {}", &source[node.byte_range()]))?;

    let props = extract_props(source, node, options)?;
    let children = extract_children(source, node, options)?;

    if options.jsx_automatic {
        // React 17+ automatic runtime
        transform_to_jsx_runtime(&tag_name, &props, &children)
    } else {
        // Classic React.createElement
        transform_to_create_element(&tag_name, &props, &children, options)
    }
}

/// Transform JSX fragment <>...</>
fn transform_jsx_fragment(source: &str, node: &Node, options: &TransformOptions) -> Result<String> {
    let children = extract_children(source, node, options)?;

    if options.jsx_automatic {
        Ok(format!("jsxs(Fragment, {{ children: [{}] }})", children.join(", ")))
    } else {
        let fragment = options.jsx_fragment.as_deref().unwrap_or("React.Fragment");
        Ok(format!(
            "{}({}, null, {})",
            options.jsx_pragma.as_deref().unwrap_or("React.createElement"),
            fragment,
            children.join(", ")
        ))
    }
}

/// Extract tag name from JSX element
///
/// ✅ Fix Bug #3: Returns Option instead of defaulting to "div"
fn extract_tag_name(source: &str, node: &Node) -> Option<String> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "jsx_opening_element" => {
                return extract_tag_from_opening(&child, source);
            }
            "identifier" | "member_expression" => {
                // Self-closing element
                return Some(source[child.byte_range()].to_string());
            }
            _ => {}
        }
    }

    // ✅ Return None instead of default "div"
    tracing::warn!("Could not extract tag name from JSX element: {}", &source[node.byte_range()]);
    None
}

/// Extract tag name from opening element
fn extract_tag_from_opening(opening: &Node, source: &str) -> Option<String> {
    let mut cursor = opening.walk();

    for child in opening.children(&mut cursor) {
        match child.kind() {
            "identifier" | "member_expression" => {
                return Some(source[child.byte_range()].to_string());
            }
            _ => {}
        }
    }

    tracing::warn!("Could not extract tag name from opening element: {}", &source[opening.byte_range()]);
    None
}

/// Extract props from JSX element
fn extract_props(source: &str, node: &Node, options: &TransformOptions) -> Result<Vec<(String, String)>> {
    let mut cursor = node.walk();

    // Find jsx_opening_element
    for child in node.children(&mut cursor) {
        if child.kind() == "jsx_opening_element" {
            return extract_props_from_opening(source, &child, options);
        }
    }

    Ok(vec![])
}

/// Extract props from opening element
fn extract_props_from_opening(source: &str, opening: &Node, options: &TransformOptions) -> Result<Vec<(String, String)>> {
    let mut props = vec![];
    let mut cursor = opening.walk();

    for child in opening.children(&mut cursor) {
        if child.kind() == "jsx_attribute" {
            let prop = extract_single_prop(source, &child, options)?;
            props.push(prop);
        }
    }

    Ok(props)
}

/// Extract a single prop
fn extract_single_prop(source: &str, attr: &Node, options: &TransformOptions) -> Result<(String, String)> {
    let mut cursor = attr.walk();
    let mut name = String::new();
    let mut value = String::from("true"); // Default for boolean props

    for child in attr.children(&mut cursor) {
        match child.kind() {
            "property_identifier" => {
                name = source[child.byte_range()].to_string();
            }
            "jsx_expression" => {
                // Extract expression inside {}
                value = extract_jsx_expression(source, &child, options)?;
            }
            "string" => {
                value = source[child.byte_range()].to_string();
            }
            _ => {}
        }
    }

    Ok((name, value))
}

/// Extract JSX expression (content inside {})
fn extract_jsx_expression(source: &str, node: &Node, options: &TransformOptions) -> Result<String> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "{" | "}" => continue,
            "jsx_element" | "jsx_self_closing_element" => {
                return transform_jsx_element(source, &child, options);
            }
            _ => {
                if child.child_count() > 0 {
                    return transform_node(source, &child, options);
                } else {
                    return Ok(source[child.byte_range()].to_string());
                }
            }
        }
    }

    Ok(String::new())
}

/// Extract children from JSX element
fn extract_children(source: &str, node: &Node, options: &TransformOptions) -> Result<Vec<String>> {
    let mut children = vec![];
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "jsx_text" => {
                let text = source[child.byte_range()].trim();
                if !text.is_empty() {
                    children.push(format!("\"{}\"", text.replace('"', "\\\"")));
                }
            }
            "jsx_expression" => {
                let expr = extract_jsx_expression(source, &child, options)?;
                if !expr.is_empty() {
                    children.push(expr);
                }
            }
            "jsx_element" | "jsx_self_closing_element" => {
                children.push(transform_jsx_element(source, &child, options)?);
            }
            "jsx_fragment" => {
                children.push(transform_jsx_fragment(source, &child, options)?);
            }
            _ => {}
        }
    }

    Ok(children)
}

/// Transform to React 17+ jsx() runtime
fn transform_to_jsx_runtime(tag_name: &str, props: &[(String, String)], children: &[String]) -> Result<String> {
    let is_component = tag_name.chars().next().unwrap_or('a').is_uppercase();
    let tag = if is_component {
        tag_name.to_string()
    } else {
        format!("\"{}\"", tag_name)
    };

    let jsx_func = if children.len() > 1 { "jsxs" } else { "jsx" };

    let mut props_str = String::new();
    if !props.is_empty() {
        for (key, value) in props {
            if !props_str.is_empty() {
                props_str.push_str(", ");
            }
            props_str.push_str(&format!("{}: {}", key, value));
        }
    }

    if !children.is_empty() {
        if !props_str.is_empty() {
            props_str.push_str(", ");
        }
        if children.len() == 1 {
            props_str.push_str(&format!("children: {}", children[0]));
        } else {
            props_str.push_str(&format!("children: [{}]", children.join(", ")));
        }
    }

    if props_str.is_empty() {
        Ok(format!("{}({}, {{}})", jsx_func, tag))
    } else {
        Ok(format!("{}({}, {{ {} }})", jsx_func, tag, props_str))
    }
}

/// Transform to classic React.createElement()
fn transform_to_create_element(
    tag_name: &str,
    props: &[(String, String)],
    children: &[String],
    options: &TransformOptions,
) -> Result<String> {
    let is_component = tag_name.chars().next().unwrap_or('a').is_uppercase();
    let tag = if is_component {
        tag_name.to_string()
    } else {
        format!("\"{}\"", tag_name)
    };

    let pragma = options.jsx_pragma.as_deref().unwrap_or("React.createElement");

    let props_obj = if props.is_empty() {
        "null".to_string()
    } else {
        let mut props_str = String::new();
        for (key, value) in props {
            if !props_str.is_empty() {
                props_str.push_str(", ");
            }
            props_str.push_str(&format!("{}: {}", key, value));
        }
        format!("{{ {} }}", props_str)
    };

    if children.is_empty() {
        Ok(format!("{}({}, {})", pragma, tag, props_obj))
    } else {
        Ok(format!("{}({}, {}, {})", pragma, tag, props_obj, children.join(", ")))
    }
}

/// Generate source map (placeholder)
fn generate_source_map(_source: &str) -> String {
    r#"{"version":3,"sources":[],"names":[],"mappings":""}"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tsx_simple_component() {
        let source = r#"const App: React.FC = () => <div>Hello</div>;"#;
        let options = TransformOptions::default();
        let result = transform_tsx(source, &options).unwrap();

        // Should remove type annotation
        assert!(!result.code.contains(": React.FC"));
        // Should transform JSX
        assert!(result.code.contains("jsx(\"div\""));
        assert!(result.code.contains("Hello"));
    }

    #[test]
    fn test_tsx_nested_jsx() {
        let source = r#"
const App = () => (
  <React.StrictMode>
    <App />
  </React.StrictMode>
);"#;
        let options = TransformOptions::default();
        let result = transform_tsx(source, &options).unwrap();

        // Should transform both elements
        assert!(result.code.contains("React.StrictMode"));
        assert!(result.code.contains("App"));
        assert!(!result.code.contains("<React.StrictMode>"));
    }

    #[test]
    fn test_tsx_function_call_with_jsx() {
        let source = r#"root.render(<App />);"#;
        let options = TransformOptions::default();
        let result = transform_tsx(source, &options).unwrap();

        // Should preserve function call structure
        assert!(result.code.contains("root.render"));
        // Should transform JSX
        assert!(result.code.contains("jsx(App"));
        assert!(!result.code.contains("<App />"));
    }

    #[test]
    fn test_tsx_with_type_annotations() {
        let source = r#"
interface Props {
    name: string;
}

const Greeting: React.FC<Props> = ({ name }: Props) => <div>Hello {name}</div>;
"#;
        let options = TransformOptions::default();
        let result = transform_tsx(source, &options).unwrap();

        // Debug: print the transformed code
        println!("Transformed code:\n{}", result.code);

        // Should remove interface
        assert!(!result.code.contains("interface Props"));
        // Should remove type annotations
        assert!(!result.code.contains(": React.FC"));
        assert!(!result.code.contains(": Props"));
        // Should transform JSX (jsxs because it has multiple children)
        assert!(result.code.contains("jsxs(\"div\"") || result.code.contains("jsx(\"div\""));
    }
}
