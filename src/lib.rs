use reqwest::Client;
use std::fs::File;
use std::io::{self, BufReader, Cursor, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::task;
use zip::write::{FileOptions, ZipWriter};

pub struct StreamingUploader<'a> {
  client: Client,
  server_url: &'a str,
  zip_file_name: &'a str,
  total_bytes_uploaded: u64,
  total_size: Option<u64>,
  tasks: Arc<Mutex<Vec<task::JoinHandle<()>>>>,
}

impl<'a> StreamingUploader<'a> {
  pub fn new(
    client: Client,
    server_url: &'a str,
    zip_file_name: &'a str,
  ) -> Self {
    Self {
      client,
      server_url,
      zip_file_name,
      total_bytes_uploaded: 0,
      total_size: None,
      tasks: Arc::new(Mutex::new(Vec::new())),
    }
  }

  pub fn upload_chunk(
    &mut self,
    data: &[u8],
    is_last_chunk: bool,
  ) -> io::Result<()> {

    // Calculate Content-Range header
    let total_size_header = if is_last_chunk {
      self.total_bytes_uploaded + data.len() as u64
    } else {
      0
    };
    let end_byte = self.total_bytes_uploaded + data.len() as u64 - 1;
    let content_range = if is_last_chunk {
      format!(
        "bytes {}-{}/{}",
        self.total_bytes_uploaded, end_byte, total_size_header
      )
    } else {
      format!(
        "bytes {}-{}/{}",
        self.total_bytes_uploaded, end_byte, "*"
      )
    };

    // Clone data required for the task
    let client = self.client.clone();
    let server_url = self.server_url.to_string();
    let zip_file_name = self.zip_file_name.to_string();
    let chunk_data = data.to_vec();
    
    // Spawn the upload task
    let task = task::spawn(async move {
      // Perform the upload asynchronously
      match client
        .post(&server_url)
        .header("x-filename", zip_file_name)
        .header("Content-Range", content_range)
        .header("Content-Length", chunk_data.len().to_string())
        .body(chunk_data)
        .send()
        .await
      {
        Ok(resp) => {
          if resp.status().is_success() {
            println!("✅ Chunk uploaded successfully!");
          } else {
            let status = resp.status();
            let response_body =
              resp.text().await.unwrap_or_else(|_| {
                "Failed to read response body".to_string()
              });
            println!(
              "❌ Upload failed: {}. Response body: {}",
              status, response_body
            );
          }
        }
        Err(e) => println!("❌ Network error: {}", e),
      }
    });

    // Add the task to the task list
    self.tasks.lock().unwrap().push(task);

    // Update total bytes uploaded
    self.total_bytes_uploaded += data.len() as u64;
    Ok(())
  }

  pub async fn wait_for_all_uploads(&self) {
    let tasks = {
      let mut locked_tasks = self.tasks.lock().unwrap();
      std::mem::take(&mut *locked_tasks)
    };

    // Wait for all tasks to complete
    futures::future::join_all(tasks).await;

    println!("✅ All uploads completed.");
  }
}

pub struct ChunkedStreamingUploader<'a> {
  uploader: StreamingUploader<'a>,
  intermediate_buffer: Cursor<Vec<u8>>,
  chunk_size: usize,
}

impl<'a> ChunkedStreamingUploader<'a> {
  pub fn new(
    uploader: StreamingUploader<'a>,
    chunk_size: usize,
  ) -> Self {
    Self {
      uploader,
      intermediate_buffer: Cursor::new(Vec::with_capacity(
        chunk_size,
      )),
      chunk_size,
    }
  }

  pub fn flush_to_server(&mut self) -> io::Result<()> {
    if self.intermediate_buffer.position() > 0 {
      let data = self.intermediate_buffer.get_ref();
      let is_last_chunk = data.len() < self.chunk_size;

      self.uploader.upload_chunk(data, is_last_chunk)?;

      // Clear the buffer
      self.intermediate_buffer.set_position(0);
      self.intermediate_buffer.get_mut().clear();
    }
    Ok(())
  }
}

impl<'a> Write for ChunkedStreamingUploader<'a> {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    self.intermediate_buffer.write_all(buf)?;
    if self.intermediate_buffer.position() as usize >= self.chunk_size
    {
      self.flush_to_server()?;
    }
    Ok(buf.len())
  }

  fn flush(&mut self) -> io::Result<()> {
    self.uploader.total_size =
      Some(self.uploader.total_bytes_uploaded);
    self.flush_to_server()
  }
}

impl<'a> Seek for ChunkedStreamingUploader<'a> {
  fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
    self.intermediate_buffer.seek(pos)
  }
}

pub async fn compress_and_upload_streaming(
  path_to_compress: &str,
  zip_file_name: &str,
  server_url: &str,
  chunk_size: usize,
) -> io::Result<()> {
  let client = Client::new();

  // Initialize the streaming uploader
  let uploader =
    StreamingUploader::new(client, server_url, zip_file_name);

  // Create a chunked streaming uploader
  let mut writer =
    ChunkedStreamingUploader::new(uploader, chunk_size);

  // Initialize the ZIP writer with the intermediate writer
  let mut zip = ZipWriter::new(&mut writer);

  let path = Path::new(path_to_compress);
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

  zip.finish()?; // Finalize the ZIP archive
  writer.flush()?; // Ensure all remaining data is uploaded

  writer.uploader.wait_for_all_uploads().await;

  Ok(())
}

fn add_directory_to_zip<W: Write + Seek>(
  zip: &mut ZipWriter<W>,
  dir_path: &Path,
  base_path: &Path,
) -> io::Result<()> {
  for entry in std::fs::read_dir(dir_path)? {
    let entry = entry?;
    let entry_path = entry.path();

    if entry_path.is_file() {
      add_file_to_zip(zip, &entry_path, base_path)?;
    } else if entry_path.is_dir() {
      add_directory_to_zip(zip, &entry_path, base_path)?;
    }
  }
  Ok(())
}

fn add_file_to_zip<W: Write + Seek>(
  zip: &mut ZipWriter<W>,
  file_path: &Path,
  base_path: &Path,
) -> io::Result<()> {
  let file = File::open(file_path)?;
  let relative_path = file_path.strip_prefix(base_path).unwrap();
  let options: FileOptions<()> =
    FileOptions::default().large_file(true);
  zip.start_file(relative_path.to_str().unwrap(), options)?;

  io::copy(&mut BufReader::new(file), zip)?;

  Ok(())
}
