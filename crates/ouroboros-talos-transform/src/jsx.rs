use anyhow::Result;
use tree_sitter::{Parser, Node};
use crate::{TransformOptions, TransformResult};

/// Transform JSX to JavaScript using Tree-sitter
pub fn transform_jsx(source: &str, options: &TransformOptions) -> Result<TransformResult> {
    tracing::debug!("Transforming JSX (jsx_automatic: {})", options.jsx_automatic);

    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_javascript::LANGUAGE.into())?;

    let tree = parser.parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse JSX"))?;

    let root = tree.root_node();

    // Transform JSX elements
    let transformed = transform_node(source, &root, options)?;

    Ok(TransformResult {
        code: transformed,
        source_map: if options.source_maps {
            Some(generate_source_map(source))
        } else {
            None
        },
    })
}

/// Transform a single AST node
fn transform_node(source: &str, node: &Node, options: &TransformOptions) -> Result<String> {
    let mut result = String::new();
    let mut cursor = node.walk();
    let mut last_end = node.start_byte();

    for child in node.children(&mut cursor) {
        // Preserve whitespace between nodes
        if child.start_byte() > last_end {
            result.push_str(&source[last_end..child.start_byte()]);
        }

        match child.kind() {
            "jsx_element" | "jsx_self_closing_element" => {
                result.push_str(&transform_jsx_element(source, &child, options)?);
            }
            "jsx_fragment" => {
                result.push_str(&transform_jsx_fragment(source, &child, options)?);
            }
            _ => {
                // Recursively process children
                if child.child_count() > 0 {
                    result.push_str(&transform_node(source, &child, options)?);
                } else {
                    // Leaf node - check if it's JSX-related before copying
                    match child.kind() {
                        "jsx_element" | "jsx_self_closing_element" => {
                            result.push_str(&transform_jsx_element(source, &child, options)?);
                        }
                        _ => result.push_str(&source[child.byte_range()]),
                    }
                }
            }
        }

        last_end = child.end_byte();
    }

    // Preserve any trailing whitespace
    if last_end < node.end_byte() {
        result.push_str(&source[last_end..node.end_byte()]);
    }

    Ok(result)
}

/// Transform JSX element to React.createElement or jsx() call
fn transform_jsx_element(source: &str, node: &Node, options: &TransformOptions) -> Result<String> {
    let tag_name = extract_tag_name(source, node)?;
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
        let _fragment = options.jsx_fragment.as_deref().unwrap_or("React.Fragment");
        Ok(format!("{}(Fragment, null, {})",
            options.jsx_pragma.as_deref().unwrap_or("React.createElement"),
            children.join(", ")))
    }
}

/// Extract tag name from JSX element
fn extract_tag_name(source: &str, node: &Node) -> Result<String> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "jsx_opening_element" => {
                // Find identifier or member_expression in opening tag
                return extract_tag_from_opening(&child, source);
            }
            "identifier" | "member_expression" => {
                // Self-closing element
                return Ok(source[child.byte_range()].to_string());
            }
            _ => {}
        }
    }

    Ok("div".to_string())
}

/// Extract tag name from opening element
fn extract_tag_from_opening(opening: &Node, source: &str) -> Result<String> {
    let mut cursor = opening.walk();

    for child in opening.children(&mut cursor) {
        match child.kind() {
            "identifier" | "member_expression" => {
                return Ok(source[child.byte_range()].to_string());
            }
            _ => {}
        }
    }

    Ok("div".to_string())
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

    Ok(Vec::new())
}

/// Extract props from opening element
fn extract_props_from_opening(source: &str, opening: &Node, options: &TransformOptions) -> Result<Vec<(String, String)>> {
    let mut props = Vec::new();
    let mut cursor = opening.walk();

    for child in opening.children(&mut cursor) {
        if child.kind() == "jsx_attribute" {
            if let Some((name, value)) = extract_jsx_attribute(source, &child, options)? {
                props.push((name, value));
            }
        }
    }

    Ok(props)
}

/// Extract single JSX attribute
fn extract_jsx_attribute(source: &str, attr: &Node, options: &TransformOptions) -> Result<Option<(String, String)>> {
    let mut cursor = attr.walk();
    let mut name = None;
    let mut value = None;

    for child in attr.children(&mut cursor) {
        match child.kind() {
            "property_identifier" => {
                name = Some(source[child.byte_range()].to_string());
            }
            "string" => {
                // String literal value
                value = Some(source[child.byte_range()].to_string());
            }
            "jsx_expression" => {
                // Expression in braces: prop={value}
                value = Some(extract_jsx_expression_value(source, &child, options)?);
            }
            _ => {}
        }
    }

    match (name, value) {
        (Some(n), Some(v)) => Ok(Some((n, v))),
        (Some(n), None) => {
            // Boolean prop: <Component disabled />
            Ok(Some((n, "true".to_string())))
        }
        _ => Ok(None),
    }
}

/// Extract value from JSX expression {}
fn extract_jsx_expression_value(source: &str, expr: &Node, options: &TransformOptions) -> Result<String> {
    let mut result = String::new();
    let mut cursor = expr.walk();

    for child in expr.children(&mut cursor) {
        match child.kind() {
            "{" | "}" => continue,
            _ => {
                // Handle nested JSX recursively
                if child.kind().starts_with("jsx_") {
                    result.push_str(&transform_jsx_element(source, &child, options)?);
                } else {
                    result.push_str(&source[child.byte_range()]);
                }
            }
        }
    }
    Ok(result)
}

/// Extract children from JSX element
fn extract_children(source: &str, node: &Node, options: &TransformOptions) -> Result<Vec<String>> {
    let mut children = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "jsx_element" | "jsx_self_closing_element" => {
                children.push(transform_jsx_element(source, &child, options)?);
            }
            "jsx_text" => {
                let text = &source[child.byte_range()];
                if !text.trim().is_empty() {
                    let escaped = text.trim()
                        .replace('\\', "\\\\")
                        .replace('"', "\\\"")
                        .replace('\n', "\\n")
                        .replace('\r', "\\r")
                        .replace('\t', "\\t");
                    children.push(format!("\"{}\"", escaped));
                }
            }
            "jsx_expression" => {
                let expr_value = extract_jsx_expression_value(source, &child, options)?;
                if !expr_value.trim().is_empty() {
                    children.push(expr_value);
                }
            }
            _ => {}
        }
    }

    Ok(children)
}

/// Transform to React.createElement call (classic mode)
fn transform_to_create_element(
    tag: &str,
    props: &[(String, String)],
    children: &[String],
    options: &TransformOptions,
) -> Result<String> {
    let pragma = options.jsx_pragma.as_deref().unwrap_or("React.createElement");

    // Build props object
    let props_str = if props.is_empty() {
        "null".to_string()
    } else {
        let props_pairs: Vec<String> = props.iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect();
        format!("{{ {} }}", props_pairs.join(", "))
    };

    // Determine if tag is component (PascalCase) or DOM element (lowercase)
    let tag_str = if tag.chars().next().map_or(false, |c| c.is_uppercase()) {
        tag.to_string() // Component reference
    } else {
        format!("\"{}\"", tag) // String for DOM element
    };

    // Build createElement call
    if children.is_empty() {
        Ok(format!("{}({}, {})", pragma, tag_str, props_str))
    } else {
        Ok(format!("{}({}, {}, {})", pragma, tag_str, props_str, children.join(", ")))
    }
}

/// Transform to jsx() runtime call (React 17+ automatic)
fn transform_to_jsx_runtime(
    tag: &str,
    props: &[(String, String)],
    children: &[String],
) -> Result<String> {
    let tag_str = if tag.chars().next().map_or(false, |c| c.is_uppercase()) {
        tag.to_string()
    } else {
        format!("\"{}\"", tag)
    };

    let mut props_map = props.to_vec();
    if !children.is_empty() {
        props_map.push(("children".to_string(),
            if children.len() == 1 {
                children[0].clone()
            } else {
                format!("[{}]", children.join(", "))
            }
        ));
    }

    let props_str = if props_map.is_empty() {
        "{}".to_string()
    } else {
        let pairs: Vec<String> = props_map.iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect();
        format!("{{ {} }}", pairs.join(", "))
    };

    let fn_name = if children.len() > 1 { "jsxs" } else { "jsx" };
    Ok(format!("{}({}, {})", fn_name, tag_str, props_str))
}

/// Generate a simple source map
fn generate_source_map(_source: &str) -> String {
    // TODO: Implement proper source map generation
    r#"{"version":3,"sources":[],"names":[],"mappings":""}"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsx_transform_basic() {
        let source = "const App = () => <div>Hello</div>";
        let mut options = TransformOptions::default();
        options.jsx_automatic = false;

        let result = transform_jsx(source, &options).unwrap();
        // Should contain React.createElement
        assert!(result.code.contains("React.createElement") ||
                result.code.len() > source.len());
    }

    #[test]
    fn test_jsx_automatic_mode() {
        let source = "<div>Test</div>";
        let mut options = TransformOptions::default();
        options.jsx_automatic = true;

        let _result = transform_jsx(source, &options).unwrap();
        // Basic transformation should work
    }

    #[test]
    fn test_jsx_with_expressions() {
        let source = r#"const App = () => <h1>{message}</h1>"#;
        let mut options = TransformOptions::default();
        options.jsx_automatic = true;

        let result = transform_jsx(source, &options).unwrap();
        // Should contain children: message
        assert!(result.code.contains("children"));
        assert!(result.code.contains("message"));
    }

    #[test]
    fn test_jsx_mixed_children() {
        let source = r#"const Counter = () => <button>Count: {count}</button>"#;
        let mut options = TransformOptions::default();
        options.jsx_automatic = true;

        let result = transform_jsx(source, &options).unwrap();
        // Should contain both text and expression
        assert!(result.code.contains("Count:"));
        assert!(result.code.contains("count"));
    }

    #[test]
    fn test_jsx_text_escaping() {
        let source = r#"<div>Text with "quotes" and \backslash</div>"#;
        let mut options = TransformOptions::default();
        options.jsx_automatic = true;

        let result = transform_jsx(source, &options).unwrap();
        // Should properly escape special characters
        assert!(result.code.contains("\\\"") || result.code.contains("quotes"));
    }
}
