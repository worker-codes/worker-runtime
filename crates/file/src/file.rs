use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::PathBuf;
use std::time::Duration;
use std::time;

// use zbox::DirEntry;
use zbox::File as ZoboxFile;
use zbox::FileType;
// use zbox::Metadata;
// use anyhow::Result;
use zbox::Repo;
// use zbox::Version;

// use crate::error::type_error;
// use crate::error::AnyError;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use zbox::{init_env, RepoOpener};
use wapc_codec::messagepack::{deserialize, serialize};
pub mod file;
use quaint::single::Quaint;
use zbox::Repo;
use std::sync::{Arc, Mutex};
use wapc::{ResourceTable, Resource};
use serde::{Deserialize, Serialize};
use wapc_codec::messagepack::{deserialize, serialize};
use file::*;
use zbox::File as ZoboxFile;
use std::borrow::Cow;
use std::io::{Write, Read, SeekFrom, Seek};

#[derive(Deserialize, Serialize, Debug)]
struct ToFrom{
    to: String,
    from: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct CreateFile{
    path: String,
    content: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug)]
struct ReadFile{
    path: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct ReadOpenFile{
    rid: u32,
    buffer_size: u64,
}
#[derive(Deserialize, Serialize, Debug)]
struct ReadOpenFileResponse{
    size: u64,
    buffer: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug)]
struct SeekOpenFile{
    rid: u32,
    position: u64,
}
// #[derive(Deserialize, Serialize, Debug)]
// struct SeekpenFileResponse{
//     position: u64,
//     buffer: Vec<u8>,
// }

#[derive(Deserialize, Serialize, Debug)]
struct WriteOpenFile{
    rid: u32,
    content: Vec<u8>,
}


#[derive(Deserialize, Serialize, Debug)]
struct CloseOpenFile{
    rid: u32,
}

#[derive(Deserialize, Serialize, Debug)]
struct OpenFileResponse{
    rid: u32,
    file: Vec<u8>,
}


struct FileResource(Arc<Mutex<ZoboxFile>>);

impl Resource for FileResource {
    fn name(&self) -> Cow<str> {
        "ZoboxFile".into()
    }
}

pub fn process_database(
    id: u64,
    binding: &str,
    namespace: &str,
    operation: &str,
    payload: &[u8],
    resource_table: Arc<Mutex<ResourceTable>>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {

    // let repo = resource_table.lock()?.get::<Repo>(id)?;
    let repo = create_repository("id".to_string(), "resource_table".to_string())?;  

    match (binding, namespace, operation) {
        ("storage", "file", "open") => {
            let file: ReadFile = deserialize(payload)?;          
            let result = open_file(repo, file.path)?;
            let repo = resource_table.lock().unwrap();
            let rid = repo.add( FileResource(Arc::new(Mutex::new(result.0))));

            let response = serialize(&OpenFileResponse{rid, file: serialize(&result.1)?})?;
            Ok(response)        
        },
        ("storage", "file", "open_read") => {
            let write_file: ReadOpenFile = deserialize(payload)?;          

            let table = resource_table.lock().unwrap();
            let file = table.get::<FileResource>(write_file.rid).unwrap();
            let file = file.0.lock().unwrap();
            let mut buffer:Vec<u8> = Vec::with_capacity(write_file.buffer_size as usize);
            let result = file.read(&mut &buffer)?;

            let response = serialize(&ReadOpenFileResponse{size: result as u64, buffer})?;

            Ok(response)        
        },
        ("storage", "file", "open_write") => {
            let write_file: WriteOpenFile = deserialize(payload)?;          

            let table = resource_table.lock().unwrap();
            let file = table.get::<FileResource>(write_file.rid).unwrap();
            let file = file.0.lock().unwrap();
            let result = file.write(&write_file.content)?;

            let result = result as u64;
            let response = serialize(&result)?;
            Ok(response)        
        },
        ("storage", "file", "open_seek") => {
            let seek_file: SeekOpenFile = deserialize(payload)?;          

            let table = resource_table.lock().unwrap();
            let file = table.get::<FileResource>(seek_file.rid).unwrap();
            let file = file.0.lock().unwrap();
            let result = file.seek(SeekFrom::Start(seek_file.position))?;

            let response = serialize(&result)?;
            Ok(response)        
        },
        ("storage", "file", "open_close") => {
            let close_file: CloseOpenFile = deserialize(payload)?;          

            let table = resource_table.lock().unwrap();
            let file = table.get::<FileResource>(close_file.rid).unwrap();
            let file = file.0.lock().unwrap();
            let result = file.finish()?;
            table.close(close_file.rid).unwrap();

            Ok(vec![])      
        },
        ("storage", "file", "create_file") => {
            let file: CreateFile = deserialize(payload)?;       
            let result = create_file(repo, file.path, file.content)?;

            let response = serialize(&result)?;
            Ok(response) 
        },
        ("storage", "file", "read_file") => {

            let file: ReadFile = deserialize(payload)?;       
            let result = read_file(repo, file.path)?;

            let response = serialize(&result)?;
            Ok(response)
        },
        ("storage", "file", "read_text_file") => {

            let file: ReadFile = deserialize(payload)?;          
            let result = read_text_file(repo, file.path)?;

            let response = serialize(&result)?;
            Ok(response)        
        },
        ("storage", "file", "remove_file") => {

            let file: ReadFile = deserialize(payload)?;          
            let result = remove_file(repo, file.path)?;

            Ok(vec![])                 
        },
        ("storage", "file", "rename") => {
            let paths: ToFrom = deserialize(payload)?;          
            let result = rename(repo, paths.from, paths.to)?;

            Ok(vec![])          
        },
        ("storage", "file", "is_file") => {
            let file: ReadFile = deserialize(payload)?;          
            let result = is_file(repo, file.path)?;

            Ok(result)        
        },
        ("storage", "file", "is_dir") => {
            let file: ReadFile = deserialize(payload)?;          
            let result = is_dir(repo, file.path)?;

            Ok(result)        
        },
        ("storage", "file", "path_exists") => {
            let file: ReadFile = deserialize(payload)?;          
            let result = path_exists(repo, file.path)?;

            Ok(result)        
        },
        ("storage", "file", "metadata") => {
            let file: ReadFile = deserialize(payload)?;          
            let result = metadata(repo, file.path)?;

            Ok(result)        
        },
        ("storage", "file", "history") => {
            let file: ReadFile = deserialize(payload)?;          
            let result = history(repo, file.path)?;

            Ok(result)        
        },
        ("storage", "file", "create_dir") => {
            let file: ReadFile = deserialize(payload)?;          
            let result = create_dir(repo, file.path)?;

            Ok(vec![])         
        },
        ("storage", "file", "create_dir_all") => {
            let file: ReadFile = deserialize(payload)?;          
            let result = create_dir_all(repo, file.path)?;

            Ok(vec![])         
        },
        ("storage", "file", "read_dir") => {
            let file: ReadFile = deserialize(payload)?;          
            let result = read_dir(repo, file.path)?;

            Ok(result)        
        },
        ("storage", "file", "remove_dir") => {
            let file: ReadFile = deserialize(payload)?;          
            let result = remove_dir(repo, file.path)?;

            Ok(vec![])          
        },
        ("storage", "file", "remove_dir_all") => {
            let file: ReadFile = deserialize(payload)?;          
            remove_dir_all(repo, file.path)?;

            Ok(vec![])          
        },
        ("storage", "file", "copy_file") => {
            let paths: ToFrom = deserialize(payload)?;          
            copy_file(repo, paths.from, paths.to)?;

            Ok(vec![])          
        },
        ("storage", "file", "copy_dir_all") => {
            let paths: ToFrom = deserialize(payload)?;          
            copy_dir_all(repo, paths.from, paths.to)?;

            Ok(vec![])        
        },

    }
}


// #[derive(Deserialize, Serialize, Debug)]
// pub enum FileTypeOut {
//   File,
//   Dir,
// }
#[derive(Deserialize, Serialize, Debug)]
pub struct DirEntry {
  path: PathBuf,
  name: String,
  metadata: Metadata,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct Metadata {
  file_type: FileType,
  is_dir: bool,
  is_file: bool,
  content_len: usize,
  curr_version: usize,
  created_at: time::SystemTime,
  modified_at: time::SystemTime,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct Time(Duration);

#[derive(Deserialize, Serialize, Debug)]
struct File {
  path: String,
  name: String,
  content_len: u64,
  file_type: String,
  content: Vec<u8>,
  created_at: time::SystemTime,
  modified_at: time::SystemTime,
}

pub fn create_repository(repo_name: String, password: String) -> Result<Repo> {
    init_env();
    let mut repo = RepoOpener::new().create(true).open(&repo_name, &password)?;

    Ok(repo)
}

pub fn open_file(mut repo: Repo, path: String) -> Result<(ZoboxFile, File)> {
  let mut file:ZoboxFile = repo.open_file(path)?;
  let metadata = file.metadata()?;
  // get file name from path
  let file_name = get_file_name(&path); 

  let result = File {
    file_type: "file".to_string(),
    content_len: metadata.content_len() as u64,
    created_at: metadata.created_at(),
    modified_at: metadata.modified_at(),
    path,
    name: file_name,
    content:vec![]
  };

  // let response = serialize(&result)?;

  Ok((file, result))
}

fn get_file_name(path: &String) -> String {
    let file_name = PathBuf::from(*path).file_name();
    let file_name = if let Some(file_name) = file_name {
    let file_name = file_name.to_str();
    if let Some(file_name) = file_name {
      let file_name = file_name.to_string();
      file_name;
    } 
    "".to_string()
      } else {
    "".to_string()
      };
    file_name
}

pub fn create_file(mut repo: Repo, path: String, content: Vec<u8>) -> Result<Vec<u8>> {
    let mut file = repo.create_file(path)?;
    file.write_once(&content)?;

    let metadata = file.metadata()?;
    // get file name from path
    let file_name = get_file_name(&path);

    let result = File {
      file_type: "file".to_string(),
      content_len: metadata.content_len() as u64,
      created_at: metadata.created_at(),
      modified_at: metadata.modified_at(),
      path,
      name: file_name,
      content
    };

    let response = serialize(&result)?;
    Ok(response)

}

pub fn read_file(mut repo: Repo, path: String) -> Result<Option<Vec<u8>>> {
    let mut file = repo.open_file(path)?;

    let mut content = Vec::new();
    // file.seek(SeekFrom::Start(0))?;
    file.read_to_end(&mut content)?;

    Ok(Some(content))
}

pub fn remove_file(mut repo: Repo, path: String) -> Result<()> {
  let mut file = repo.remove_file(path)?;

  Ok(file)
}

pub fn read_text_file(mut repo: Repo, path: String) -> Result<Option<String>> {
    let mut file = repo.open_file(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;



    Ok(Some(content))
}

pub fn copy_file(mut repo: Repo, from: String, to: String) -> Result<()> {
  let dirs = repo.copy(from, to)?;

  Ok(dirs)
}

pub fn rename(mut repo: Repo, from: String, to: String) -> Result<()> {
  let dirs = repo.rename(from, to)?;

  Ok(dirs)
}



pub fn is_file(mut repo: Repo, path: String) -> Result<Vec<u8>> {
  let dirs = repo.is_file(path)?;

  let mut buf:Vec<u8> = vec![];
  let val = rmp::encode::write_bool(&mut buf, dirs)?;
  Ok(buf)
}

pub fn is_dir(mut repo: Repo, path: String)  -> Result<Vec<u8>> {
  let dirs = repo.is_dir(path)?;

  let mut buf:Vec<u8> = vec![];
  let val = rmp::encode::write_bool(&mut buf, dirs)?;
  Ok(buf)
}

pub fn path_exists(mut repo: Repo, path: String) -> Result<Vec<u8>> {
  let dirs = repo.path_exists(path)?;

  let mut buf:Vec<u8> = vec![];
  let val = rmp::encode::write_bool(&mut buf, dirs)?;
  Ok(buf)
}

pub fn metadata(mut repo: Repo, path: String) -> Result<Vec<u8>>{
  let metadata = repo.metadata(path)?;

  let result = Metadata {
    file_type: metadata.file_type(),
    content_len: metadata.content_len(),
    curr_version: metadata.curr_version(),
    created_at: metadata.created_at(),
    modified_at: metadata.modified_at(),
    is_dir: metadata.is_dir(),
    is_file: metadata.is_file(),
  };

  let response = serialize(&result)?;

  Ok(response)
}

pub fn history(mut repo: Repo, path: String) -> Result<Vec<u8>> {
  let dirs = repo.history(path)?;

  let response = serialize(&dirs)?;
  Ok(response)
}

pub fn create_dir(mut repo: Repo, path: String) -> Result<()> {
  let dirs = repo.create_dir(path)?;

  Ok(dirs)
}

pub fn create_dir_all(mut repo: Repo, path: String) -> Result<()> {
  let dirs = repo.create_dir_all(path)?;

  Ok(dirs)
}

pub fn read_dir(mut repo: Repo, path: String) -> Result<Vec<u8>> {
  let dirs = repo.read_dir(path)?;

  let dir_entry:Vec<DirEntry> = dirs.into_iter()
    .map(|entry| {
      let metadata = entry.metadata();
      DirEntry {
      path: entry.path().to_path_buf(),
      name: entry.file_name().to_string(),
      metadata: Metadata {
        file_type: metadata.file_type(),
        content_len: metadata.content_len(),
        curr_version: metadata.curr_version(),
        created_at: metadata.created_at(),
        modified_at: metadata.modified_at(),
        is_dir: metadata.is_dir(),
        is_file: metadata.is_file(),
      },
    }
  
  })
    .collect();

    let response = serialize(&dir_entry)?;
  Ok(response)
}

pub fn remove_dir_all(mut repo: Repo, path: String) -> Result<()> {
  let dirs = repo.remove_dir_all(path)?;

  Ok(dirs)
}


pub fn remove_dir(mut repo: Repo, path: String) -> Result<()> {
  let dirs = repo.remove_dir(path)?;

  Ok(dirs)
}

pub fn copy_dir_all(mut repo: Repo, from: String, to: String) -> Result<()> {
  let dirs = repo.copy_dir_all(from, to)?;

  Ok(dirs)
}
