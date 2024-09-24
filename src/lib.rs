use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use zip::write::FileOptions;
use zip::ZipWriter;

pub fn upload_chunks(path_str: &str, zip_file_path: &str) -> io::Result<()> {
    let path = Path::new(path_str);

    let zip_file = File::create(zip_file_path)?;
    let mut zip = ZipWriter::new(zip_file);

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_file() {
                compress_file_into_zip(&mut zip, entry_path, path_str)?;
            } else if entry_path.is_dir() {
                compress_directory_into_zip(&mut zip, entry_path, path_str)?;
            }
        }
    } else if path.is_file() {
        compress_file_into_zip(&mut zip, path.to_path_buf(), path_str)?;
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid file or directory path",
        ));
    }

    zip.finish()?;

    println!("Zip file created successfully at {}", zip_file_path);

    Ok(())
}

fn compress_file_into_zip(
    zip: &mut ZipWriter<File>,
    file_path: PathBuf,
    base_dir: &str,
) -> io::Result<()> {
    let file = File::open(&file_path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = vec![0; 1024 * 1024 * 1024]; // 1GB buffer

    let relative_path = file_path.strip_prefix(base_dir).unwrap();
    let file_name = relative_path.to_str().unwrap();

    zip.start_file(file_name, FileOptions::default())?;

    loop {
        let bytes_read = reader.read(&mut buffer)?;

        if bytes_read == 0 {
            break; // End of file reached
        }

        zip.write_all(&buffer[..bytes_read])?;
    }

    println!("Compressed file: {}", file_name);

    Ok(())
}

fn compress_directory_into_zip(
    zip: &mut ZipWriter<File>,
    dir_path: PathBuf,
    base_dir: &str,
) -> io::Result<()> {
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let entry_path = entry.path();

        if entry_path.is_file() {
            compress_file_into_zip(zip, entry_path, base_dir)?;
        } else if entry_path.is_dir() {
            compress_directory_into_zip(zip, entry_path, base_dir)?;
        }
    }

    Ok(())
}
