use std::ffi::OsString;
use std::fs;
use std::io;
use std::io::Read;

use classfile_parser::class_parser;
use classfile_parser::constant_info::ConstantInfo;
use failure::format_err;
use failure::Error;
use failure::ResultExt;
use zip::ZipArchive;

fn main() -> Result<(), Error> {
    for path in std::env::args_os().skip(1) {
        if let Err(e) = handle(&path) {
            eprintln!("error: {:?}: {:?} -- {:?}", path, e, e.backtrace());
        }
    }
    Ok(())
}

fn handle(path: &OsString) -> Result<(), Error> {
    let mut archive = zip::ZipArchive::new(io::BufReader::new(
        fs::File::open(&path).with_context(|_| format_err!("opening"))?,
    ))
    .with_context(|_| format_err!("unzipping"))?;

    let mut buf = Vec::with_capacity(8 * 1024);

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .with_context(|_| format_err!("opening item {}", i))?;
        if !file.name().ends_with(".class") {
            continue;
        }

        buf.clear();
        file.read_to_end(&mut buf)?;

        let super_class =
            read_super(&buf).with_context(|_| format_err!("super of {:?}", file.name()))?;
        println!("{:?}\t{:?}\t{:?}", path, file.name(), super_class);
    }

    Ok(())
}

fn read_super(from: &[u8]) -> Result<String, Error> {
    match class_parser(from) {
        Ok((&[], class_file)) => {
            match class_file
                .const_pool
                .get(usize::from(class_file.super_class - 1))
            {
                Some(ConstantInfo::Class(cc)) => {
                    match class_file.const_pool.get(usize::from(cc.name_index - 1)) {
                        Some(ConstantInfo::Utf8(d)) => Ok(d.utf8_string.to_string()),
                        other => Err(format_err!("non-utf8 parent? {:?}", other)),
                    }
                }
                other => Err(format_err!("non-class parent? {:?}", other)),
            }
        }
        err => Err(format_err!("parse explosion: {:?}", err)),
    }
}
