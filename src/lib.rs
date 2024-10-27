use reqwest::blocking::Client;
use std::fs::File;
use std::fs::{self};
use std::io::{self, BufReader, Cursor, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use zip::write::FileOptions;
use zip::ZipWriter;

pub fn compress_and_upload_in_chunks(
  path_to_compress: &str,
  zip_file_name: &str,
  server_url: &str,
) -> io::Result<()> {
  let start_time = Instant::now();
  let path = Path::new(path_to_compress);
  let client = Client::new();

  // Create a buffer to hold zip data temporarily.
  let mut zip_data = Vec::new();

  {
    let mut zip = ZipWriter::new(Cursor::new(&mut zip_data));

    // Compress the directory or file.
    if path.is_dir() {
      add_directory_to_zip(&mut zip, path, path)?;
    } else if path.is_file() {
      add_file_to_zip(&mut zip, path, path)?;
    } else {
      return Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "Invalid file or directory path",
      ));
    }

    // Finalize the zip archive.
    zip.finish()?;
  }

  // Upload the zip data in chunks
  let mut total_bytes_uploaded = 0;
  let buffer_size = 1024 * 1024; // 1MB buffer
  let mut cursor = Cursor::new(&zip_data);

  loop {
    let mut buffer = vec![0; buffer_size];
    let bytes_read = cursor.read(&mut buffer)?;

    if bytes_read == 0 {
      break; // End of zip data
    }

    let start_byte = total_bytes_uploaded;
    let end_byte = total_bytes_uploaded + bytes_read as u64 - 1;
    let content_range =
      format!("bytes {}-{}/{}", start_byte, end_byte, zip_data.len());

    // Upload the current chunk
    let response = client
      .post(server_url)
      .header("x-filename", zip_file_name)
      .header("Content-Range", content_range)
      .header("Content-Length", bytes_read.to_string())
      .body(buffer.clone())
      .send()
      .expect("Failed to upload chunk");

    if !response.status().is_success() {
      eprintln!("Upload failed with status: {:?}", response.status());
      return Err(io::Error::new(
        io::ErrorKind::Other,
        "Upload failed",
      ));
    }

    total_bytes_uploaded += bytes_read as u64;
  }

  println!("Uploaded compressed zip: {}", zip_file_name);
  println!("Total  time taken {:?}", start_time.elapsed());

  Ok(())
}

// Recursively adds a directory and its files to the zip archive.
fn add_directory_to_zip<W: Write + Seek>(
  zip: &mut ZipWriter<W>,
  dir_path: &Path,
  base_path: &Path,
) -> io::Result<()> {
  for entry in fs::read_dir(dir_path)? {
    let entry = entry?;
    let entry_path = entry.path();

    if entry_path.is_file() {
      println!("Adding file to zip: {:?}", entry_path);
      add_file_to_zip(zip, &entry_path, base_path)?;
    } else if entry_path.is_dir() {
      println!("Entering directory: {:?}", entry_path);
      add_directory_to_zip(zip, &entry_path, base_path)?;
    }
  }
  Ok(())
}

// Adds a single file to the zip archive.
fn add_file_to_zip<W: Write + Seek>(
  zip: &mut ZipWriter<W>,
  file_path: &Path,
  base_path: &Path,
) -> io::Result<()> {
  let file = File::open(file_path)?;
  let relative_path = file_path.strip_prefix(base_path).unwrap();
  zip.start_file(
    relative_path.to_str().unwrap(),
    FileOptions::default(),
  )?;

  let mut reader = BufReader::new(file);
  let mut buffer = [0; 4096];
  loop {
    let bytes_read = reader.read(&mut buffer)?;
    if bytes_read == 0 {
      break;
    }
    zip.write_all(&buffer[..bytes_read])?;
  }
  Ok(())
}
