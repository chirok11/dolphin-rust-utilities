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
async fn archivate_folder(output_file: String, input_dir: String, file_list: Vec<String>) -> Result<bool> {
    let handle = tokio::task::spawn(async move {
        let output_path = Path::new(&output_file);
        let input_dir = Path::new(&input_dir);

        let file = File::create(&output_path)?;
        let mut zip_writer = ZipWriter::new(file);
        let options = FileOptions::default()
            .compression_method(CompressionMethod::Deflated);

        for file in file_list {
            if file.contains('*') {
                for entry in glob(input_dir.join(file).to_str().unwrap()).unwrap() {
                    match entry {
                        Ok(pathb) => {
                            let pathn = pathb.strip_prefix(input_dir).map_err(
                                |e| napi::Error::new(GenericFailure, format!("Error: {:?}", &e))
                            )?;
                            debug!("adding: {:?}", pathn);
                            if pathb.is_dir() {
                                zip_writer.add_directory(path_to_string(pathn), options).map_err(
                                    |e| napi::Error::new(GenericFailure, format!("Error: {:?}", &e))
                                )?;
                                continue;
                            }
                            let m = zip_writer.start_file(path_to_string(pathn), options);
                            if m.is_err() {
                                return Err(ZipError::WriteError(pathn.to_path_buf()).into())
                            }

                            let data = get_bytes_by_filename(&pathb);
                            if data.is_err() { return Err(ZipError::FileReadError(&pathn.to_path_buf()).into()); }

                            if zip_writer.write_all(get_bytes_by_filename(&pathb)?.as_slice()).is_err() {
                                return Err(ZipError::WriteError(pathn.to_path_buf()).into())
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
    }).await.map_err(|e| {
        napi::Error::new(GenericFailure, format!("{}", e))
    })?;

    handle
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

#[test]
fn test_glob() {
    for entry in glob("data_dir/Default/Extensions/**").unwrap() {
        println!("{:?}", entry);
    }
}