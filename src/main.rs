use rust_upload::compress_and_upload_streaming;
use std::time::Instant;

#[tokio::main]
async fn main() {
  let start_time = Instant::now();

  let path_to_compress = "src/test";
  let file_name = "test.zip";
  let server_url = "http://localhost:3000/upload";
  let chunk_size = 1024 * 1024 * 1024; // 1 GB chunks

  // Run the compression and upload process
  match compress_and_upload_streaming(
    path_to_compress,
    file_name,
    server_url,
    chunk_size,
  ) {
    Ok(_) => println!("Compressed file uploaded successfully!"),
    Err(e) => eprintln!("Compression failed: {:?}", e),
  }

  // Measure end time and calculate the duration
  let duration = start_time.elapsed();
  println!("Script ran for: {:.2?}", duration);
}
