use reqwest::blocking::Client;
use std::fs::File;
use std::fs::{self};
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use zip::write::FileOptions;
use zip::ZipWriter;

pub fn compress_and_upload_in_chunks(
  path_to_compress: &str,
  file_name: &str,
  server_url: &str,
) -> io::Result<()> {
  let path = Path::new(path_to_compress);
  let client = Client::new();

  if path.is_dir() {
    for entry in fs::read_dir(path)? {
      let entry = entry?;
      let entry_path = entry.path();

      if entry_path.is_file() {
        compress_file_and_upload(
          &client,
          server_url,
          entry_path,
          path_to_compress,
          file_name,
        )?;
      } else if entry_path.is_dir() {
        compress_directory_into_zip(
          &client,
          server_url,
          entry_path,
          path_to_compress,
          file_name,
        )?;
      }
    }
  } else if path.is_file() {
    compress_file_and_upload(
      &client,
      server_url,
      path.to_path_buf(),
      path_to_compress,
      file_name,
    )?;
  } else {
    return Err(io::Error::new(
      io::ErrorKind::InvalidInput,
      "Invalid file or directory path",
    ));
  }

  Ok(())
}

fn compress_file_and_upload(
  client: &Client,
  server_url: &str,
  entry_path: PathBuf,
  path_to_compress: &str,
  zip_file_name: &str,
) -> io::Result<()> {
  let file = File::open(&entry_path)?;
  let mut reader = BufReader::new(file);
  let mut buffer = vec![0; 1024 * 1024 * 1024]; // 1GB buffer (you can adjust the size)

  let relative_path =
    entry_path.strip_prefix(path_to_compress).unwrap();
  let file_name = relative_path.to_str().unwrap();

  let mut zip_data = Vec::new();
  let mut total_bytes_uploaded = 0;
  let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_data));

  loop {
    let bytes_read = reader.read(&mut buffer)?;

    if bytes_read == 0 {
      break; // End of file reached
    }

    zip.start_file(file_name, FileOptions::default())?;
    zip.write_all(&buffer[..bytes_read])?;
    zip.flush()?;

    let is_last_chunk = bytes_read < buffer.len();

    // Calculate the correct Content-Range and total size for this chunk
    let start_byte = total_bytes_uploaded;
    let end_byte = total_bytes_uploaded + zip_data.len() as u64 - 1;

    let total_size = if is_last_chunk {
      (total_bytes_uploaded + zip_data.len() as u64).to_string() // This is the total size for the last chunk
    } else {
      "*".to_string()
    };

    // Set the Content-Range with the total bytes for this chunk
    let content_range =
      format!("bytes {}-{}/{}", start_byte, end_byte, total_size);

    let response = client
      .post(server_url)
      .header("x-filename", zip_file_name)
      .header("Content-Range", content_range)
      .header("Content-Length", zip_data.len().to_string())
      .body(zip_data.clone())
      .send()
      .expect("Failed to upload chunk");

    if !response.status().is_success() {
      eprintln!("Upload failed with status: {:?}", response.status());
    }

    total_bytes_uploaded += zip_data.len() as u64;
    zip_data.clear();
  }

  println!("Uploaded compressed file: {}", file_name);

  Ok(())
}

fn compress_directory_into_zip(
  client: &Client,
  server_url: &str,
  dir_path: PathBuf,
  path_to_compress: &str,
  zip_file_name: &str,
) -> io::Result<()> {
  for entry in fs::read_dir(dir_path)? {
    let entry = entry?;
    let entry_path = entry.path();

    if entry_path.is_file() {
      compress_file_and_upload(
        &client,
        server_url,
        entry_path,
        path_to_compress,
        zip_file_name,
      )?;
    } else if entry_path.is_dir() {
      compress_directory_into_zip(
        &client,
        server_url,
        entry_path,
        path_to_compress,
        zip_file_name,
      )?;
    }
  }

  Ok(())
}
