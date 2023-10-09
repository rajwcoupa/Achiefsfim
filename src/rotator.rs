// Copyright (C) 2023, Achiefs.

use std::fs::{metadata, File, copy, read_to_string, remove_file, create_dir};
use std::io::{BufReader, BufRead, Write};
use std::path::Path;
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use std::thread;
use log::{debug, error, info};

use crate::config;
use crate::utils;

// ----------------------------------------------------------------------------

// Compress given file into zip.
fn compress_file(filepath: &str) -> Result<String, String> {
    if utils::get_os() == "windows"{
        use zip::write::FileOptions;

        let filename = Path::new(filepath).file_name().unwrap().to_str().unwrap();
        let zipfilename = format!("{}.zip", filepath);
        let path = Path::new(&zipfilename);
        let zipfile = File::create(path).unwrap();
    
        let mut zip = zip::ZipWriter::new(zipfile);
        match zip.start_file(filename, FileOptions::default()){
            Ok(_v) => {
                let file = File::open(filepath).unwrap();
                let reader = BufReader::new(file);

                let mut counter: u64 = 0;
                let mut broken = false;
                for line in reader.lines() {
                    // Sleep during compression to avoid increase CPU load.
                    if counter == 4096 {
                        thread::sleep(Duration::from_millis(500));
                        counter = 0;
                    }
                    match zip.write_all(line.unwrap().as_bytes()){
                        Ok(_v) => debug!("Line written into zip file from rotated file."),
                        Err(e) => {
                            error!("Error writting line to zip file, error: {}", e);
                            broken = true;
                            break;
                        }
                    };
                    counter += 1;
                }
                if ! broken {
                    match zip.finish(){
                        Ok(_v) => debug!("File compressed successfully, result: {}", zipfilename),
                        Err(e) => error!("Error compressing rotated file, error:{}", e)
                    };
                }
                Ok(format!("File {} compressed successfully", filename))
            },
            Err(e) => {
                error!("Could not create file inside zip file, error: {}", e);
                Err(format!("{}", e))
            }
        }
    }else{ Ok(String::from("OK")) }
}

// ----------------------------------------------------------------------------

fn get_iteration(filepath: &str) -> u32{
    let mut path = Path::new(filepath).parent().unwrap().to_path_buf();
    path.push(Path::new("archive"));
    
    path.read_dir().expect("read_dir call failed").count() as u32
}

// ----------------------------------------------------------------------------

fn rotate_file(filepath: &str, iteration: u32, lock: &mut bool){
    info!("Rotating {} file...", filepath);
    *lock = true;
    thread::sleep(Duration::new(15, 0));
    let path = Path::new(filepath);
    let mut parent_path = path.parent().unwrap().to_path_buf();
    parent_path.push(Path::new("archive"));
    parent_path.push(Path::new(path.file_name().unwrap()));

    let file_rotated = format!("{}.{}",
        parent_path.to_str().unwrap(), iteration);
    
    match copy(filepath, file_rotated.clone()){
        Ok(_v) => {
            debug!("File copied successfully.");
            match File::create(filepath){
                Ok(truncated_file) => {
                    debug!("File truncated successfully.");
                    let tmp_file = format!("{}.tmp", filepath);
                    let data = match read_to_string(tmp_file.clone()){
                        Ok(read_data) => read_data,
                        Err(_e) => {
                            debug!("No temporal data to copy.");
                            String::new()
                        }
                    };
                    match write!(&truncated_file, "{}", data){
                        Ok(_v) => debug!("Temporal file data written into destination file."),
                        Err(e) => error!("Cannot write temporal data to destination file skipping, error: {}", e)
                    };
                    match remove_file(tmp_file){
                        Ok(_v) => debug!("Temporal file removed successfully."),
                        Err(e) => error!("Cannot remove temporal file skipping, error: {}", e)
                    };
                },
                Err(e) => error!("Error truncating file, retrying on next iteration, error: {}", e)
            };
        },
        Err(e) => error!("File cannot be copied, retrying on next iteration, error: {}", e)
    };

    *lock = false;
    info!("File {} rotated.", filepath);
    info!("Compressing rotated file {}", file_rotated);
    match compress_file(&file_rotated){
        Ok(message) => {
            info!("{}", message);
            match remove_file(file_rotated.clone()) {
                Ok(_v) => info!("File {} removed.", file_rotated),
                Err(e) => error!("Cannot remove rotated file, error: {}", e)
            }
        },
        Err(e) => error!("Error compressing file, error: {}", e)
    };
    
    
}

// ----------------------------------------------------------------------------

pub fn rotator(){
    let config = unsafe { super::GCONFIG.clone().unwrap() };
    let mut start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

    loop{
        if (start_time + Duration::new(10, 0)).as_millis() < SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis() {
            let log_size = metadata(config.clone().log_file).unwrap().len() as usize;
            let events_size = metadata(config.clone().events_file).unwrap().len() as usize;

            if events_size >= config.events_max_file_size * 1000000 {
                let events_path = Path::new(config.events_file.as_str());
                let mut parent_path = events_path.parent().unwrap().to_path_buf();
                parent_path.push(Path::new("archive"));
    
                if ! parent_path.exists(){
                    match create_dir(parent_path){
                        Ok(_v) => debug!("Archive directory created successfully."),
                        Err(e) => error!("Cannot create archive directory, error: {}", e)
                    };
                }

                unsafe { rotate_file(config.clone().events_file.as_str(),
                    get_iteration(config.clone().events_file.as_str()), &mut config::TMP_EVENTS) };
            }

            if log_size >= config.log_max_file_size * 1000000 {
                unsafe { rotate_file(config.clone().log_file.as_str(),
                    get_iteration(config.clone().log_file.as_str()), &mut config::TMP_LOG) };
            }

            start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        }
    }
}

// ----------------------------------------------------------------------------

