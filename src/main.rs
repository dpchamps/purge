use clap::Parser;
use std::fs;
use std::fs::{DirEntry, Metadata};
use std::time::{Duration, SystemTime};

#[derive(Parser, Debug)]
struct Args {
    /// Age of file in ms
    #[arg(long)]
    ttl: usize,

    /// Directory to purge
    #[arg(long)]
    directory: String,

    #[arg(long, default_value_t = false)]
    dry_run: bool
}

fn select_candidate_from_maybe_dir_entry(
    metadata: Result<Metadata, std::io::Error>,
    compare_to: SystemTime,
    ttl: usize,
) -> bool {
    match metadata
        .and_then(|x| x.created())
        .and_then(|x| compare_to.duration_since(x).map_err(|x| std::io::Error::other(x)))
    {
        Ok(duration_since_creation) => duration_since_creation.as_millis() > ttl as u128,
        Err(_) => false,
    }
}

fn extract_paths_to_delete(directory: String, ttl: usize) -> Result<Vec<(String, Metadata)>, ()> {
    let now = SystemTime::now();
    fs::read_dir(directory)
        .expect("Unable to find directory")
        .filter(|maybe_dir_entry| match maybe_dir_entry {
            Ok(dir_entry) => select_candidate_from_maybe_dir_entry(dir_entry.metadata(), now, ttl),
            Err(_) => false,
        })
        .map(|dir_entry| match dir_entry {
            Ok(dir_entry) => dir_entry.path().into_os_string().into_string().map(|x| (x, dir_entry.metadata().expect(""))).map_err(|_| ()),
            Err(_) => Err(())
        })
        .collect()
}

fn delete_path((path, metadata): (String, Metadata)) -> Result<(), std::io::Error> {
    if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else if metadata.is_file() {
        fs::remove_file(path)
    } else {
        Ok(())
    }
}

fn main() {
    let args = Args::parse();
    let paths = extract_paths_to_delete(args.directory, args.ttl).expect("Could not look at paths");

    println!("Files to purge: {}", paths.iter().map(|(p, _)| &**p).collect::<Vec<&str>>().join(", "));
    if !args.dry_run {
        let (_, errors): (Vec<_>, Vec<_>) = paths.into_iter().map(delete_path).partition(Result::is_ok);
        if !errors.is_empty() {
            eprintln!("Failed to delete the following: {:?}", errors);
        }
    }
}
