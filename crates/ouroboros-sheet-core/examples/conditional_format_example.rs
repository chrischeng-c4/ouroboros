use ouroboros_sheet_core::{
    Cell, CellCoord, CellFormat, CellRange, Color, ComparisonOperator, ConditionalFormat,
    ConditionalFormattingRule, ConditionalRule, Sheet, TextOperator,
};

fn main() {
    let mut sheet = Sheet::new("Sales Data");

    // Add some sample data
    sheet.set_cell(CellCoord::new(0, 0), Cell::text("Product"));
    sheet.set_cell(CellCoord::new(0, 1), Cell::text("Sales"));
    sheet.set_cell(CellCoord::new(1, 0), Cell::text("Widget A"));
    sheet.set_cell(CellCoord::new(1, 1), Cell::number(75.0));
    sheet.set_cell(CellCoord::new(2, 0), Cell::text("Widget B"));
    sheet.set_cell(CellCoord::new(2, 1), Cell::number(45.0));
    sheet.set_cell(CellCoord::new(3, 0), Cell::text("Widget C"));
    sheet.set_cell(CellCoord::new(3, 1), Cell::number(90.0));
    sheet.set_cell(CellCoord::new(4, 0), Cell::text("Widget D"));
    sheet.set_cell(CellCoord::new(4, 1), Cell::number(30.0));

    // Example 1: Value-based rule - highlight high sales in green
    println!("Example 1: Value-based conditional formatting");
    let high_sales_rule = ConditionalFormattingRule::new(
        "high_sales".to_string(),
        CellRange::new(CellCoord::new(1, 1), CellCoord::new(10, 1)),
        ConditionalRule::ValueBased {
            operator: ComparisonOperator::GreaterThanOrEqual,
            value1: 70.0,
            value2: None,
            format: ConditionalFormat {
                background_color: Some(Color::rgb(144, 238, 144)), // Light green
                bold: Some(true),
                ..Default::default()
            },
        },
    );
    sheet.add_conditional_formatting(high_sales_rule);

    // Example 2: Value-based rule - highlight low sales in red
    let low_sales_rule = ConditionalFormattingRule::new(
        "low_sales".to_string(),
        CellRange::new(CellCoord::new(1, 1), CellCoord::new(10, 1)),
        ConditionalRule::ValueBased {
            operator: ComparisonOperator::LessThan,
            value1: 50.0,
            value2: None,
            format: ConditionalFormat {
                background_color: Some(Color::rgb(255, 182, 193)), // Light red
                italic: Some(true),
                ..Default::default()
            },
        },
    );
    sheet.add_conditional_formatting(low_sales_rule);

    // Example 3: Text-based rule - highlight product names containing "A"
    println!("\nExample 2: Text-based conditional formatting");
    let text_rule = ConditionalFormattingRule::new(
        "contains_a".to_string(),
        CellRange::new(CellCoord::new(1, 0), CellCoord::new(10, 0)),
        ConditionalRule::TextBased {
            operator: TextOperator::Contains,
            pattern: Some("A".to_string()),
            case_sensitive: false,
            format: ConditionalFormat {
                text_color: Some(Color::rgb(0, 0, 255)), // Blue
                underline: Some(true),
                ..Default::default()
            },
        },
    );
    sheet.add_conditional_formatting(text_rule);

    // Example 4: Color scale - create a gradient based on sales values
    println!("\nExample 3: Color scale conditional formatting");

    // For demonstration, let's create a separate sheet for color scale
    let mut color_sheet = Sheet::new("Color Scale Example");
    color_sheet.set_cell(CellCoord::new(0, 0), Cell::number(0.0));
    color_sheet.set_cell(CellCoord::new(0, 1), Cell::number(25.0));
    color_sheet.set_cell(CellCoord::new(0, 2), Cell::number(50.0));
    color_sheet.set_cell(CellCoord::new(0, 3), Cell::number(75.0));
    color_sheet.set_cell(CellCoord::new(0, 4), Cell::number(100.0));

    let color_scale_rule = ConditionalFormattingRule::new(
        "sales_gradient".to_string(),
        CellRange::new(CellCoord::new(0, 0), CellCoord::new(0, 4)),
        ConditionalRule::ColorScale {
            min_color: Color::rgb(255, 0, 0),     // Red for low values
            max_color: Color::rgb(0, 255, 0),     // Green for high values
            mid_color: Some(Color::rgb(255, 255, 0)), // Yellow for mid values
        },
    );
    color_sheet.add_conditional_formatting(color_scale_rule);

    // Test effective format calculation
    println!("\nTesting effective format calculation:");
    let base_format = CellFormat::default();

    // Widget A (sales: 75) - should be green and bold (high sales)
    let value = sheet.get_cell_value(CellCoord::new(1, 1));
    let effective = sheet.get_effective_format(1, 1, &base_format, value);
    println!("Widget A (75): Bold={}, Background={:?}",
        effective.bold,
        effective.background_color.map(|c| c.to_hex())
    );

    // Widget D (sales: 30) - should be light red and italic (low sales)
    let value = sheet.get_cell_value(CellCoord::new(4, 1));
    let effective = sheet.get_effective_format(4, 1, &base_format, value);
    println!("Widget D (30): Italic={}, Background={:?}",
        effective.italic,
        effective.background_color.map(|c| c.to_hex())
    );

    // Widget A name - should be blue and underlined (contains "A")
    let value = sheet.get_cell_value(CellCoord::new(1, 0));
    let effective = sheet.get_effective_format(1, 0, &base_format, value);
    println!("Widget A name: Underline={}, TextColor={:?}",
        effective.underline,
        effective.text_color.map(|c| c.to_hex())
    );

    // Test color scale
    println!("\nColor scale values:");
    for col in 0..5 {
        let value = color_sheet.get_cell_value(CellCoord::new(0, col));
        let effective = color_sheet.get_effective_format(0, col, &base_format, value);
        if let Some(color) = effective.background_color {
            println!("Value {}: Color={}", value.as_number().unwrap(), color.to_hex());
        }
    }

    // Show all conditional formatting rules
    println!("\nAll conditional formatting rules on sheet:");
    for rule in sheet.get_conditional_formatting_rules() {
        println!("  - Rule ID: {}, Range: {}, Enabled: {}, Priority: {}",
            rule.id,
            rule.range.to_a1(),
            rule.enabled,
            rule.priority
        );
    }

    // Example of removing a rule
    println!("\nRemoving 'low_sales' rule...");
    sheet.remove_conditional_formatting("low_sales");
    println!("Remaining rules: {}", sheet.get_conditional_formatting_rules().len());
}
