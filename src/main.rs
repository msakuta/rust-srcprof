//! The source profiler program.
//!
//! It lists up all files in a directory tree and sum up line counts
//! for making statistics analysis.

use std::{
    collections::HashSet,
    env,
    ffi::OsString,
    fs::File,
    io::{BufRead, BufReader, Result},
    path::{Path, PathBuf},
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    root: Option<PathBuf>,
    #[structopt(short = "l", long)]
    no_listing: bool,
    #[structopt(short = "h", long = "html")]
    enable_html: bool,
    #[structopt(short, long)]
    extensions: Vec<String>,
}

fn main() -> Result<()> {
    let settings: Settings = Opt::from_args().into();

    eprintln!(
        "Searching path: {:?} extensions: {:?}",
        settings.root, settings.extensions
    );
    let files = std::fs::read_dir(&settings.root)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if !entry.file_type().ok()?.is_file() {
                return None;
            }
            let path = entry.path();
            let ext = path.extension().or_else(|| path.file_name())?;
            if !settings.extensions.contains(&ext.to_ascii_lowercase()) {
                return None;
            }
            Some(Ok(path))
        })
        .collect::<Result<Vec<_>>>()?;
    eprintln!("Listing {} files...", files.len());
    process_file_list(&settings.root, &files, &settings);
    Ok(())
}

#[derive(Debug)]
struct Settings {
    root: PathBuf,
    listing: bool,
    enable_html: bool,
    extensions: HashSet<OsString>,
}

// It's a bit awkward to convert from Opt to Settings, but some settings are hard to write
// conversion code inside structopt annotations.
impl From<Opt> for Settings {
    fn from(src: Opt) -> Self {
        let default_exts = [
            ".sh", ".js", ".tcl", ".pl", ".py", ".rb", ".c", ".cpp", ".h", ".rc", ".rci", ".dlg",
            ".pas", ".dpr", ".cs", ".rs",
        ];
        Self {
            root: src
                .root
                .unwrap_or_else(|| PathBuf::from(env::current_dir().unwrap().to_str().unwrap())),
            listing: !src.no_listing,
            enable_html: src.enable_html,
            extensions: if src.extensions.is_empty() {
                default_exts.iter().map(|ext| ext[1..].into()).collect()
            } else {
                default_exts
                    .iter()
                    .map(|ext| ext[1..].into())
                    .chain(src.extensions.iter().map(|ext| ext[1..].into()))
                    .collect()
            },
        }
    }
}

fn process_file_list(root: &Path, files: &[PathBuf], settings: &Settings) {
    for (i, f) in files.iter().enumerate() {
        // let ext = if let Some(ext) = f.extension().or_else(|| f.file_name()) {
        //     ext.to_ascii_lowercase()
        // } else {
        //     continue;
        // };

        // if !settings.extensions.contains(&ext) {
        //     continue;
        // }

        let filepath = root.join(f);
        let fp = match File::open(&filepath) {
            Ok(fp) => fp,
            Err(e) => {
                eprintln!("Failed to open {:?}: {:?}", filepath, e);
                continue;
            }
        };
        let reader = BufReader::new(fp).lines();
        let linecount = reader.count();

        let filesize = match std::fs::metadata(&filepath) {
            Ok(meta) => meta.len(),
            Err(e) => {
                eprintln!("Failed to get metadata for {:?}: {:?}", filepath, e);
                continue;
            }
        };

        if settings.listing {
            if settings.enable_html {
                println!(
                    "<tr><td>{0}</td><td>{1}</td><td>{2}</td><td>{3:?}</td></tr>",
                    i, linecount, filesize, filepath
                );
            } else {
                println!("{0}\t{1:5}\t{2:?}", i + 1, linecount, filepath);
            }
        }

        // fe = fileentry()
        // fe.name = filepath
        // fe.lines = linecount
        // fe.size = filesize
        // filelist.append((linecount, fe))
        // fp.close()

        // 	if !ext in extstats:
        // 		extstats[ext] = srcstats()
        // 	extstats[ext].lines += linecount
        // 	extstats[ext].files += 1
        // 	extstats[ext].size += filer.getsize(root, f)
        // return
    }
}
