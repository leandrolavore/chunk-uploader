use rust_upload::compress_and_upload_streaming;
use std::env;
use std::time::Instant;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 5 {
        eprintln!("Usage: uploader <path_to_compress> <zip_file_name> <server_url> <chunk_size>");
        std::process::exit(1);
    }

    let path_to_compress = &args[1];
    let zip_file_name = &args[2];
    let server_url = &args[3];
    let chunk_size: usize = args[4].parse().unwrap_or_else(|_| {
        eprintln!("Invalid chunk size");
        std::process::exit(1);
    });

    let start_time = Instant::now();

    // Run the compression and upload process
    match compress_and_upload_streaming(
        path_to_compress,
        zip_file_name,
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


// use rust_upload::compress_and_upload_streaming;
// use std::time::Instant;

// #[tokio::main]
// async fn main() {
//   let start_time = Instant::now();

//   let path_to_compress = "src/test";
//   let file_name = "test.zip";
//   let server_url = "http://localhost:3000/upload";
//   let chunk_size = 1024 * 1024 * 1024; // 1 GB chunks

//   // Run the compression and upload process
//   match compress_and_upload_streaming(
//     path_to_compress,
//     file_name,
//     server_url,
//     chunk_size,
//   ) {
//     Ok(_) => println!("Compressed file uploaded successfully!"),
//     Err(e) => eprintln!("Compression failed: {:?}", e),
//   }

//   // Measure end time and calculate the duration
//   let duration = start_time.elapsed();
//   println!("Script ran for: {:.2?}", duration);
// }
