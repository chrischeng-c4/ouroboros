//! Example: File upload and form data parsing
//!
//! This example demonstrates how to use the new form data support
//! in data-bridge-api for handling file uploads and multipart forms.

use data_bridge_api::request::{parse_multipart, parse_urlencoded, SerializableFormData};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Parse URL-encoded form data
    println!("=== URL-encoded Form Data ===");
    let urlencoded_body = b"name=Alice&email=alice%40example.com&age=30";
    let fields = parse_urlencoded(urlencoded_body)?;
    println!("Parsed fields: {:?}", fields);
    println!();

    // Example 2: Parse multipart form with file upload
    println!("=== Multipart Form with File Upload ===");
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let multipart_body = b"\
------WebKitFormBoundary7MA4YWxkTrZu0gW\r
Content-Disposition: form-data; name=\"title\"\r
\r
My Document\r
------WebKitFormBoundary7MA4YWxkTrZu0gW\r
Content-Disposition: form-data; name=\"file\"; filename=\"data.txt\"\r
Content-Type: text/plain\r
\r
Hello, World! This is file content.\r
------WebKitFormBoundary7MA4YWxkTrZu0gW\r
Content-Disposition: form-data; name=\"description\"\r
\r
Important file upload\r
------WebKitFormBoundary7MA4YWxkTrZu0gW--\r
";

    let form_data: SerializableFormData = parse_multipart(boundary.to_string(), multipart_body.to_vec()).await?;
    
    println!("Text fields:");
    for (key, value) in &form_data.fields {
        println!("  {}: {}", key, value);
    }
    
    println!("\nFile uploads:");
    for file in &form_data.files {
        println!("  Field: {}", file.field_name);
        println!("  Filename: {}", file.filename);
        println!("  Content-Type: {}", file.content_type);
        println!("  Size: {} bytes", file.data.len());
        println!("  Content: {}", String::from_utf8_lossy(&file.data));
    }

    Ok(())
}
