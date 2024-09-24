use rust_upload::upload_chunks;
use std::time::Instant;

fn main() {
    // Measure start time
    let start_time = Instant::now();

    // Your file or directory path and output ZIP file
    let path_to_compress = "src/test"; // Can be a file or directory
    let zip_file_path = "compressed.zip";

    // Run the compression
    match upload_chunks(path_to_compress, zip_file_path) {
        Ok(_) => println!("Compression completed successfully!"),
        Err(e) => eprintln!("Compression failed: {:?}", e),
    }

    // Measure end time and calculate the duration
    let duration = start_time.elapsed();
    println!("Script ran for: {:.2?}", duration);
}
