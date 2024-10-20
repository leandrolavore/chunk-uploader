use rust_upload::compress_and_upload_in_chunks;
use std::time::Instant;

fn main() {
  let start_time = Instant::now();
  let path_to_compress = "src/test";
  let file_name = "test.zip";
  let server_url = "http://localhost:3000/upload";

  // Run the compression
  match compress_and_upload_in_chunks(
    path_to_compress,
    file_name,
    server_url,
  ) {
    Ok(_) => println!("Compressed file uploaded successfully!"),
    Err(e) => eprintln!("Compression failed: {:?}", e),
  }

  // Measure end time and calculate the duration
  let duration = start_time.elapsed();
  println!("Script ran for: {:.2?}", duration);
}
