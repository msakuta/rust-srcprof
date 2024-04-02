//! The source profiler program.
//!
//! It lists up all files in a directory tree and sum up line counts
//! for making statistics analysis.

use anyhow::Result;
use dunce::canonicalize;
#[cfg(feature = "git2")]
use git2::{Repository, TreeWalkResult};
use rayon::prelude::*;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashSet},
    env,
    ffi::OsString,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
};
use structopt::StructOpt;
use walkdir::WalkDir;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(help = "Root directory to profile")]
    root: Option<PathBuf>,
    #[structopt(short = "l", long, help = "Enable listing of inspected files")]
    listing: bool,
    #[structopt(
        short = "h",
        long = "html",
        help = "Enable HTML output rather than plain text"
    )]
    enable_html: bool,
    #[structopt(
        short = "r",
        long,
        default_value = "10",
        help = "Set number of files to show in ranking of line count"
    )]
    ranking: u32,
    #[structopt(short = "s", long, help = "Show statistics summary")]
    no_summary: bool,
    #[structopt(short = "d", long, help = "Show statistics summary")]
    no_distrib: bool,
    #[cfg(feature = "git2")]
    #[structopt(short = "g", long, help = "Load from git repository")]
    use_git: bool,
    #[cfg(feature = "git2")]
    #[structopt(
        short = "b",
        long,
        help = "Git branch name to search line numbers. If omitted, HEAD is used."
    )]
    branch: Option<String>,
    #[structopt(short, long, help = "Add an entry to list of extensions to search")]
    extensions: Vec<String>,
    #[structopt(
        short,
        long,
        help = "Add an entry to list of directory names to ignore"
    )]
    ignore_dirs: Vec<String>,
    #[structopt(
        short = "H",
        long,
        help = "Show line counts and file sizes in human-readable format (e.g. 101.2K instead of 101214)"
    )]
    human_readable: bool,
    #[structopt(
        short = "u",
        long,
        help = "Count file size in utf-8 characters instead of bytes"
    )]
    utf8: bool,
}

fn main() -> Result<()> {
    let settings: Settings = Opt::from_args().into();

    eprintln!(
        "Searching path: {:?} extensions: {:?} ignore_dirs: {:?}",
        settings.root, settings.extensions, settings.ignore_dirs
    );

    #[cfg(feature = "git2")]
    let (mut file_list, extstats) = if settings.use_git {
        process_files_git(&settings.root, &settings)?
    } else {
        process_files(&settings.root, &settings)?
    };
    #[cfg(not(feature = "git2"))]
    let (mut file_list, extstats) = process_files(&settings.root, &settings)?;

    if settings.enable_html {
        println!(
            r#"
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd">
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="en">
<head>
<title>srcprof.py output</title>
</head>
<body>
"#
        )
    }

    show_summary(&settings, &extstats);

    show_listing(&settings, &mut file_list);

    show_distribution(&settings, &file_list, |v| v);

    if settings.enable_html {
        println!("</body></html>");
    }

    Ok(())
}

#[derive(Debug)]
struct Settings {
    root: PathBuf,
    listing: bool,
    enable_html: bool,
    ranking: u32,
    summary: bool,
    enable_distrib: bool,
    #[cfg(feature = "git2")]
    use_git: bool,
    #[cfg(feature = "git2")]
    branch: Option<String>,
    extensions: HashSet<OsString>,
    ignore_dirs: HashSet<OsString>,
    human_readable: bool,
    utf8: bool,
}

// It's a bit awkward to convert from Opt to Settings, but some settings are hard to write
// conversion code inside structopt annotations.
impl From<Opt> for Settings {
    fn from(src: Opt) -> Self {
        let default_exts = [
            ".sh", ".js", ".tcl", ".pl", ".py", ".rb", ".c", ".cpp", ".h", ".rc", ".rci", ".dlg",
            ".pas", ".dpr", ".cs", ".rs",
        ];
        let default_ignore_dirs = [".hg", ".svn", ".git", ".bzr", "node_modules", "target"]; // Probably we could ignore all directories beginning with a dot.

        Self {
            root: canonicalize(
                src.root.unwrap_or_else(|| {
                    PathBuf::from(env::current_dir().unwrap().to_str().unwrap())
                }),
            )
            .expect("Canonicalized path"),
            listing: src.listing,
            enable_html: src.enable_html,
            ranking: src.ranking,
            summary: !src.no_summary,
            enable_distrib: !src.no_distrib,
            #[cfg(feature = "git2")]
            use_git: src.use_git,
            #[cfg(feature = "git2")]
            branch: src.branch,
            extensions: if src.extensions.is_empty() {
                default_exts.iter().map(|ext| ext[1..].into()).collect()
            } else {
                default_exts
                    .iter()
                    .map(|ext| ext[1..].into())
                    .chain(src.extensions.iter().map(|ext| ext[1..].into()))
                    .collect()
            },
            ignore_dirs: if src.ignore_dirs.is_empty() {
                default_ignore_dirs.iter().map(|ext| ext.into()).collect()
            } else {
                default_ignore_dirs
                    .iter()
                    .map(|ext| ext.into())
                    .chain(src.ignore_dirs.iter().map(|ext| ext.into()))
                    .collect()
            },
            human_readable: src.human_readable,
            utf8: src.utf8,
        }
    }
}

struct FileEntry {
    name: PathBuf,
    lines: usize,
    size: u64,
}

fn format_human_readable(size: u64, humread: bool) -> String {
    if !humread || size < 2u64.pow(10) {
        format!("{size}")
    } else if size < 2u64.pow(20) {
        format!("{:.1}K", size as f64 / 2u64.pow(10) as f64)
    } else if size < 2u64.pow(30) {
        format!("{:.1}M", size as f64 / 2u64.pow(20) as f64)
    } else {
        format!("{:.1}G", size as f64 / 2u64.pow(20) as f64)
    }
}

#[derive(Default)]
struct SrcStats {
    files: usize,
    lines: usize,
    size: u64,
}

impl SrcStats {
    fn tostring(&self, humread: bool) -> String {
        format!(
            "files = {}, lines = {}, size = {}",
            self.files,
            format_human_readable(self.lines as u64, humread),
            format_human_readable(self.size, humread)
        )
    }

    fn tohtml(&self, humread: bool) -> String {
        format!(
            "<td>{}</td><td>{}</td><td>{}</td>",
            self.files,
            format_human_readable(self.lines as u64, humread),
            format_human_readable(self.size, humread)
        )
    }

    fn htmlheader() -> &'static str {
        "<th>file</th><th>lines</th><th>size</th>"
    }
}

type SrcStatsSet = BTreeMap<OsString, SrcStats>;

fn process_files(root: &Path, settings: &Settings) -> Result<(Vec<FileEntry>, SrcStatsSet)> {
    let mut walked = 0;
    let files = WalkDir::new(&settings.root)
        .into_iter()
        .filter_entry(|e| !e.file_type().is_dir() || !settings.ignore_dirs.contains(e.file_name()))
        .filter_map(|entry| {
            walked += 1;
            let entry = entry.ok()?;
            if !entry.file_type().is_file() {
                return None;
            }
            let path = entry.path().to_owned();
            let ext = path.extension().or_else(|| path.file_name())?;
            if !settings.extensions.contains(&ext.to_ascii_lowercase()) {
                return None;
            }
            Some(Ok(path))
        })
        .collect::<Result<Vec<_>>>()?;
    eprintln!("Listing {}/{} files...", files.len(), walked);
    Ok(process_file_list(root, &files, settings))
}

fn process_file_list(
    root: &Path,
    files: &[PathBuf],
    settings: &Settings,
) -> (Vec<FileEntry>, SrcStatsSet) {
    let mut filelist = vec![];
    let mut extstats = SrcStatsSet::new();

    if settings.listing && settings.enable_html {
        println!("<h1>File list in \"{}\"</h1>", root.to_string_lossy());
        println!(r#"<table border="1" cellspacing="0" cellpadding="1">"#);
        println!("<tr><th>No.</th><th>lines</th><th>size</th><th>name</th></tr>");
    }

    filelist.par_extend(files.par_iter().enumerate().filter_map(|(i, f)| {
        let filepath = root.join(f);
        let fp = match File::open(&filepath) {
            Ok(fp) => fp,
            Err(e) => {
                eprintln!("Failed to open {:?}: {:?}", filepath, e);
                return None;
            }
        };

        let filesize = if settings.utf8 {
            match std::fs::File::open(&filepath) {
                Ok(f) => {
                    let mut s = String::new();
                    let mut reader = std::io::BufReader::new(f);
                    if let Err(e) = reader.read_to_string(&mut s) {
                        eprintln!(
                            "Failed to read contents as a string from {:?}: {:?}",
                            filepath, e
                        );
                        return None;
                    }
                    s.chars().count() as u64
                }
                Err(e) => {
                    eprintln!("Failed to open file {:?}: {:?}", filepath, e);
                    return None;
                }
            }
        } else {
            match std::fs::metadata(&filepath) {
                Ok(meta) => meta.len(),
                Err(e) => {
                    eprintln!("Failed to get metadata for {:?}: {:?}", filepath, e);
                    return None;
                }
            }
        };

        process_file(settings, fp, filepath, i, filesize)
    }));

    for fe in &filelist {
        let ext = fe.name.extension().unwrap().to_ascii_lowercase();

        let entry = extstats.entry(ext).or_default();
        entry.lines += fe.lines;
        entry.files += 1;
        entry.size += fe.size;
    }

    if settings.listing && settings.enable_html {
        println!("</table><hr>")
    }

    (filelist, extstats)
}

fn process_file(
    settings: &Settings,
    fp: impl std::io::Read,
    filepath: PathBuf,
    i: usize,
    filesize: u64,
) -> Option<FileEntry> {
    let reader = BufReader::new(fp).lines();
    let linecount = reader.count();

    if settings.listing {
        if settings.enable_html {
            println!(
                "<tr><td>{0}</td><td>{1}</td><td>{2}</td><td>{3}</td></tr>",
                i,
                linecount,
                filesize,
                filepath.to_string_lossy()
            );
        } else {
            println!(
                "{0}\t{1:5}\t{2}",
                i + 1,
                linecount,
                filepath.to_string_lossy()
            );
        }
    }

    Some(FileEntry {
        name: filepath,
        lines: linecount,
        size: filesize,
    })
}

#[cfg(feature = "git2")]
fn process_files_git(_root: &Path, settings: &Settings) -> Result<(Vec<FileEntry>, SrcStatsSet)> {
    let mut extstats = SrcStatsSet::new();
    let mut walked = 0;
    let repo = Repository::open(&settings.root)?;
    let mut i = 0;
    let mut files = vec![];
    let reference = if let Some(ref branch) = settings.branch {
        repo.resolve_reference_from_short_name(&branch)?
    } else {
        repo.head()?
    };
    reference
        .peel_to_tree()?
        .walk(git2::TreeWalkMode::PostOrder, |_, entry| {
            match (|| {
                let name = entry.name()?;
                if entry.kind() != Some(git2::ObjectType::Blob)
                    || settings.ignore_dirs.contains(&OsString::from(name))
                {
                    return None;
                }
                let obj = match entry.to_object(&repo) {
                    Ok(obj) => obj,
                    Err(e) => {
                        eprintln!("couldn't get_object: {:?}", e);
                        return None;
                    }
                };
                let blob = obj.peel_to_blob().ok()?;
                walked += 1;
                if blob.is_binary() {
                    return None;
                }
                let path: PathBuf = entry.name()?.into();
                let ext = path.extension()?.to_owned();
                if !settings.extensions.contains(&ext.to_ascii_lowercase()) {
                    return None;
                }

                let filesize = blob.size() as u64;

                Some((
                    ext,
                    process_file(settings, blob.content(), path, i, filesize)?,
                ))
            })() {
                Some((ext, file_entry)) => {
                    let entry = extstats.entry(ext).or_default();
                    entry.lines += file_entry.lines;
                    entry.files += 1;
                    entry.size += file_entry.size;

                    files.push(file_entry);

                    i += 1;
                }
                _ => (),
            }
            TreeWalkResult::Ok
        })?;
    eprintln!("Listing {}/{} files...", files.len(), walked);
    Ok((files, extstats))
}

fn show_summary(settings: &Settings, extstats: &SrcStatsSet) {
    if !settings.summary {
        return;
    }

    if settings.enable_html {
        println!("<h1>Summary</h1>");
        println!(r#"<table border="1" cellspacing="0" cellpadding="1">"#);
        println!("<tr><th>extension</th>{}</tr>", SrcStats::htmlheader());
    } else {
        println!(
            r#"
--------------------------
     Summary
--------------------------
"#
        );
    }

    let mut extsum = SrcStats::default();
    for (ext, l) in extstats {
        if settings.enable_html {
            println!(
                r#"<tr><td>{:?}</td>{}</tr>"#,
                ext,
                l.tohtml(settings.human_readable)
            );
        } else {
            println!("{:?}: {}", ext, l.tostring(settings.human_readable));
        }
        extsum.files += l.files;
        extsum.lines += l.lines;
        extsum.size += l.size;
    }

    if settings.enable_html {
        println!(
            "<tr><td>total</td>{}</tr>",
            extsum.tohtml(settings.human_readable)
        );
    } else {
        println!("total: {}", extsum.tostring(settings.human_readable));
    }

    if settings.enable_html {
        println!("</table><hr>");
    }
}

fn show_listing(settings: &Settings, filelist: &mut [FileEntry]) {
    if 0 == settings.ranking {
        return;
    }
    if settings.enable_html {
        println!("<h1>Top {0} files</h1>", settings.ranking);
        println!(r#"<table border="1" cellspacing="0" cellpadding="1">"#);
        println!("<tr><th>No.</th><th>lines</th><th>size</th><th>name</th></tr>");
    } else {
        println!(
            r#"
--------------------------
      Top {0} files
--------------------------
"#,
            settings.ranking
        );
    }

    filelist.sort_by_key(|fe| Reverse(fe.lines));
    for (i, fe) in filelist.iter().enumerate().take(settings.ranking as usize) {
        if fe.lines == 0 {
            break;
        }
        if settings.enable_html {
            println!(
                "<tr><td>{0}</td><td>{1}</td><td>{2}</td><td>{3}</td></tr>",
                i,
                fe.lines,
                fe.size,
                fe.name.to_string_lossy()
            );
        } else {
            println!("{}: {}", fe.lines, fe.name.to_string_lossy());
        }
    }

    if settings.enable_html {
        println!("</table><hr>");
    }
}

/// `hconv` is a function to transform the bin count in the histogram.
fn show_distribution(settings: &Settings, file_list: &[FileEntry], hconv: impl Fn(f64) -> f64) {
    if !settings.enable_distrib {
        return;
    }
    let cell = 1.;
    let base = (2.0f64).sqrt();
    let mut distrib = [0; 32];
    for fe in file_list {
        if fe.lines == 0 {
            continue;
        }
        let find = (fe.lines as f64).log(base);
        let ind = find.ceil().max(0.) as usize;
        if ind < distrib.len() {
            distrib[ind] += 1;
        }
    }

    let maxdirs = hconv(distrib[1..].iter().copied().max().unwrap_or(0) as f64) as usize;

    if settings.enable_html {
        println!("<h1>Distribution</h1>");
        println!(
            r#"<table border="1" cellspacing="0" cellpadding="1">
<tr><th>Line count range</th><th>Files</th><th>Graph</th></tr>"#
        );
    } else {
        println!(
            r#"
--------------------------
      Distribution
--------------------------
"#
        );
    }

    let dist_width = if settings.enable_html { 300 } else { 60 };

    for i in 2..distrib.len() {
        let mut s = String::new();
        if settings.enable_html {
            println!(
                r#"<tr><td align="right">{0:5}-{1:5}</td><td align="right">{2:3}</td>
<td><div style="background-color:#{4:02x}007f;width:{3}px;">&nbsp;</div></td></tr>"#,
                base.powf((i as f64 - 1.) * cell).floor(),
                base.powf((i as f64) * cell).floor() - 1.,
                distrib[i],
                if distrib[i] != 0 {
                    hconv(distrib[i] as f64) as usize * dist_width / maxdirs
                } else {
                    0
                },
                (i * 255 / distrib.len()) as u32
            );
        } else {
            if 0 < maxdirs && i != 0 && distrib[i] != 0 {
                for _ in 0..hconv(distrib[i] as f64) as usize * dist_width / maxdirs {
                    s += "*"
                }
            }

            println!(
                "{0:5}-{1:5} {2:3}: {3}",
                base.powf((i as f64 - 1.) * cell).floor(),
                base.powf((i as f64) * cell).floor() - 1.,
                distrib[i],
                s
            );
        }
    }

    if settings.enable_html {
        println!("</table>");
    }
}
