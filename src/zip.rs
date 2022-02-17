use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use glob::glob;

use napi::Result;
use napi::Status::GenericFailure;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

#[derive(thiserror::Error, Debug)]
pub enum ZipError<'a> {
    #[error("unable to write file {0}")]
    WriteError(PathBuf),

    #[error("unable to read file {0}")]
    FileReadError(&'a PathBuf),

    #[error("global error occured")]
    GlobError(),
}

impl<'a> From<ZipError<'a>> for napi::Error {
    fn from(e: ZipError) -> Self {
        napi::Error::new(GenericFailure, format!("{:?}", e))
    }
}

#[napi]
fn archivate_folder(output_file: String, input_dir: String, file_list: Vec<String>) -> Result<bool> {
    let output_path = Path::new(&output_file);
    let input_dir = Path::new(&input_dir);

    let file = File::create(&output_path)?;
    let mut zip_writer = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated);

    for file in file_list {
        if file.contains('*') {
            for entry in glob(file.as_str()).unwrap() {
                match entry {
                    Ok(path) => {
                        let pathn = input_dir.join(&path);
                        debug!("writing {:?}", path);
                        if pathn.is_dir() { continue; }
                        let m = zip_writer.start_file(path_to_string(&path), options);
                        if m.is_err() {
                            return Err(ZipError::WriteError(path).into())
                        }

                        let data = get_bytes_by_filename(&path);
                        if data.is_err() { return Err(ZipError::FileReadError(&path).into()); }

                        if zip_writer.write_all(get_bytes_by_filename(&path)?.as_slice()).is_err() {
                            return Err(ZipError::WriteError(path).into())
                        };
                    },
                    Err(e) => { error!("error: {}", e); return Err(ZipError::GlobError().into()) }
                }
            }
        } else {
            if !input_dir.join(&file).is_file() { continue; }
            zip_writer.start_file(&file, options).unwrap();
            zip_writer.write_all(get_bytes_by_filename(&input_dir.join(&file))?.as_slice())?;
        }
    }

    zip_writer.finish().unwrap();

    Ok(true)
}

fn path_to_string(path: &std::path::Path) -> String {
    let mut path_str = String::new();
    for component in path.components() {
        if let std::path::Component::Normal(os_str) = component {
            if !path_str.is_empty() {
                path_str.push('/');
            }
            path_str.push_str(&*os_str.to_string_lossy());
        }
    }
    path_str
}

fn get_bytes_by_filename(path: &Path) -> Result<Vec<u8>> {
    let mut file = File::open(&path)?;

    let meta = std::fs::metadata(&path)?;
    let mut buffer = vec![0; meta.len() as usize];
    file.read_exact(&mut buffer)?;

    Ok(buffer)
}