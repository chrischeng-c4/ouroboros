use serde::{Deserialize, Serialize};

/// RGBA color representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    #[serde(default = "default_alpha")]
    pub a: u8,
}

fn default_alpha() -> u8 {
    255
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }

    /// Convert to CSS hex color string
    pub fn to_hex(&self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                self.r, self.g, self.b, self.a
            )
        }
    }

    /// Parse from CSS hex color string
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Color::rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Color::rgba(r, g, b, a))
            }
            _ => None,
        }
    }

    // Common colors
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
}

impl Default for Color {
    fn default() -> Self {
        Color::BLACK
    }
}

/// Horizontal text alignment
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HorizontalAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Vertical text alignment
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerticalAlign {
    Top,
    #[default]
    Middle,
    Bottom,
}

/// Cell formatting properties
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CellFormat {
    #[serde(default, skip_serializing_if = "is_false")]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub underline: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub strikethrough: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_color: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<Color>,
    #[serde(default, skip_serializing_if = "is_default_h_align")]
    pub horizontal_align: HorizontalAlign,
    #[serde(default, skip_serializing_if = "is_default_v_align")]
    pub vertical_align: VerticalAlign,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_format: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub wrap_text: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_default_h_align(a: &HorizontalAlign) -> bool {
    *a == HorizontalAlign::default()
}

fn is_default_v_align(a: &VerticalAlign) -> bool {
    *a == VerticalAlign::default()
}

impl CellFormat {
    /// Create a new format with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder pattern: set bold
    pub fn with_bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    /// Builder pattern: set italic
    pub fn with_italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    /// Builder pattern: set text color
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Builder pattern: set background color
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Builder pattern: set horizontal alignment
    pub fn with_horizontal_align(mut self, align: HorizontalAlign) -> Self {
        self.horizontal_align = align;
        self
    }

    /// Builder pattern: set vertical alignment
    pub fn with_vertical_align(mut self, align: VerticalAlign) -> Self {
        self.vertical_align = align;
        self
    }

    /// Builder pattern: set font size
    pub fn with_font_size(mut self, size: u8) -> Self {
        self.font_size = Some(size);
        self
    }

    /// Get the effective font size (default is 11)
    pub fn effective_font_size(&self) -> u8 {
        self.font_size.unwrap_or(11)
    }

    /// Get the effective font family (default is Arial)
    pub fn effective_font_family(&self) -> &str {
        self.font_family.as_deref().unwrap_or("Arial")
    }

    /// Merge another format into this one (other's values override)
    pub fn merge(&mut self, other: &CellFormat) {
        if other.bold {
            self.bold = true;
        }
        if other.italic {
            self.italic = true;
        }
        if other.underline {
            self.underline = true;
        }
        if other.strikethrough {
            self.strikethrough = true;
        }
        if other.font_size.is_some() {
            self.font_size = other.font_size;
        }
        if other.font_family.is_some() {
            self.font_family = other.font_family.clone();
        }
        if other.text_color.is_some() {
            self.text_color = other.text_color;
        }
        if other.background_color.is_some() {
            self.background_color = other.background_color;
        }
        if other.horizontal_align != HorizontalAlign::default() {
            self.horizontal_align = other.horizontal_align;
        }
        if other.vertical_align != VerticalAlign::default() {
            self.vertical_align = other.vertical_align;
        }
        if other.number_format.is_some() {
            self.number_format = other.number_format.clone();
        }
        if other.wrap_text {
            self.wrap_text = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_hex() {
        let color = Color::rgb(255, 128, 64);
        assert_eq!(color.to_hex(), "#ff8040");

        let parsed = Color::from_hex("#ff8040").unwrap();
        assert_eq!(parsed, color);
    }

    #[test]
    fn test_format_builder() {
        let format = CellFormat::new()
            .with_bold(true)
            .with_text_color(Color::RED)
            .with_font_size(14);

        assert!(format.bold);
        assert_eq!(format.text_color, Some(Color::RED));
        assert_eq!(format.effective_font_size(), 14);
    }
}
