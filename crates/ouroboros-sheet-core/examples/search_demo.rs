use ouroboros_sheet_core::{Cell, CellCoord, SearchEngine, SearchOptions, ReplaceOptions, Workbook};

fn main() {
    // Create a workbook with sample data
    let mut workbook = Workbook::new("SearchDemo");

    let sheet = workbook.active_sheet_mut();
    sheet.set_cell(CellCoord::new(0, 0), Cell::text("Apple"));
    sheet.set_cell(CellCoord::new(0, 1), Cell::text("Banana"));
    sheet.set_cell(CellCoord::new(0, 2), Cell::text("Cherry"));
    sheet.set_cell(CellCoord::new(1, 0), Cell::text("apple pie"));
    sheet.set_cell(CellCoord::new(1, 1), Cell::number(42.0));
    sheet.set_cell(CellCoord::new(1, 2), Cell::text("Price: $42"));
    sheet.set_cell(CellCoord::new(2, 0), Cell::formula("=SUM(B2:B3)"));

    // Example 1: Simple case-insensitive search
    println!("=== Example 1: Case-insensitive search for 'apple' ===");
    let options = SearchOptions {
        query: "apple".to_string(),
        match_case: false,
        match_entire_cell: false,
        use_regex: false,
        search_formulas: false,
        sheet_indices: None,
    };

    let results = SearchEngine::search(&workbook, &options).unwrap();
    println!("Found {} matches:", results.len());
    for result in &results {
        println!("  - Sheet: {}, Cell: {}:{} -> '{}'",
            result.sheet_name, result.row, result.col, result.cell_value);
    }

    // Example 2: Regex search for numbers
    println!("\n=== Example 2: Regex search for numbers ===");
    let options = SearchOptions {
        query: r"\d+".to_string(),
        match_case: false,
        match_entire_cell: false,
        use_regex: true,
        search_formulas: false,
        sheet_indices: None,
    };

    let results = SearchEngine::search(&workbook, &options).unwrap();
    println!("Found {} matches:", results.len());
    for result in &results {
        println!("  - Sheet: {}, Cell: {}:{} -> '{}' (matched: '{}')",
            result.sheet_name, result.row, result.col, result.cell_value, result.matched_text);
    }

    // Example 3: Search formulas
    println!("\n=== Example 3: Search in formulas ===");
    let options = SearchOptions {
        query: "SUM".to_string(),
        match_case: false,
        match_entire_cell: false,
        use_regex: false,
        search_formulas: true,
        sheet_indices: None,
    };

    let results = SearchEngine::search(&workbook, &options).unwrap();
    println!("Found {} matches:", results.len());
    for result in &results {
        println!("  - Sheet: {}, Cell: {}:{} -> '{}' (is_formula: {})",
            result.sheet_name, result.row, result.col, result.cell_value, result.is_formula);
    }

    // Example 4: Replace text
    println!("\n=== Example 4: Replace 'apple' with 'orange' ===");
    let options = ReplaceOptions {
        search: SearchOptions {
            query: "apple".to_string(),
            match_case: false,
            match_entire_cell: false,
            use_regex: false,
            search_formulas: false,
            sheet_indices: None,
        },
        replacement: "orange".to_string(),
    };

    let results = SearchEngine::replace(&mut workbook, &options).unwrap();
    println!("Replaced {} cells:", results.len());
    for result in &results {
        println!("  - Sheet: {}, Cell: {}:{} -> '{}'",
            result.sheet_name, result.row, result.col, result.cell_value);
    }

    // Verify the replacements
    println!("\n=== Verifying replacements ===");
    let sheet = workbook.active_sheet();
    println!("A1: {}", sheet.get_cell(CellCoord::new(0, 0)).unwrap().content.display_value());
    println!("A2: {}", sheet.get_cell(CellCoord::new(1, 0)).unwrap().content.display_value());

    // Example 5: Regex replace
    println!("\n=== Example 5: Replace numbers with 'XX' using regex ===");
    let options = ReplaceOptions {
        search: SearchOptions {
            query: r"\d+".to_string(),
            match_case: false,
            match_entire_cell: false,
            use_regex: true,
            search_formulas: false,
            sheet_indices: None,
        },
        replacement: "XX".to_string(),
    };

    let results = SearchEngine::replace(&mut workbook, &options).unwrap();
    println!("Replaced {} cells:", results.len());
    for result in &results {
        println!("  - Sheet: {}, Cell: {}:{} -> '{}'",
            result.sheet_name, result.row, result.col, result.cell_value);
    }
}
