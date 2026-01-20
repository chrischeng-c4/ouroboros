use anyhow::Result;
use crate::{TransformOptions, TransformResult};

/// Transform CSS to JavaScript injection code
/// This converts CSS into a JavaScript module that injects the styles into the document
pub fn transform_css(source: &str, _options: &TransformOptions) -> Result<TransformResult> {
    tracing::debug!("Transforming CSS to JS injection code");

    // Escape CSS for JavaScript template literal
    let escaped_css = source
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${");

    // Generate injection code
    let injection_code = format!(r#"// CSS Module Injection
(function() {{
  if (typeof document !== 'undefined') {{
    var style = document.createElement('style');
    style.textContent = `{}`;
    document.head.appendChild(style);
  }}
}})();
"#, escaped_css);

    Ok(TransformResult {
        code: injection_code,
        source_map: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_css_injection_code() {
        let source = ".test { color: red; }";
        let options = TransformOptions::default();
        let result = transform_css(source, &options).unwrap();

        // Should contain injection code
        assert!(result.code.contains("createElement('style')"));
        assert!(result.code.contains("appendChild"));
        assert!(result.code.contains(".test { color: red; }"));
    }

    #[test]
    fn test_css_escaping() {
        let source = r#".test { content: "hello `world` ${foo}"; }"#;
        let options = TransformOptions::default();
        let result = transform_css(source, &options).unwrap();

        // Should escape template literal special characters
        assert!(result.code.contains("\\`"));
        assert!(result.code.contains("\\${"));
    }
}
