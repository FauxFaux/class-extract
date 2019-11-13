use std::ffi::OsString;
use std::fs;
use std::io;
use std::io::Read;

use classfile_parser::class_parser;
use classfile_parser::constant_info::ConstantInfo;
use failure::format_err;
use failure::Error;
use failure::ResultExt;
use std::collections::HashSet;

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

        let (super_class, references) =
            read_file(&buf).with_context(|_| format_err!("super of {:?}", file.name()))?;
        println!(
            "{:?}\t{:?}\t{}\t{}",
            path,
            file.name(),
            super_class,
            references.join("\t")
        );
    }

    Ok(())
}

fn read_file(from: &[u8]) -> Result<(String, Vec<String>), Error> {
    match class_parser(from) {
        Ok((&[], clazz)) => {
            let parent = match clazz.const_pool.get(usize::from(clazz.super_class - 1)) {
                Some(ConstantInfo::Class(cc)) => as_str(&clazz.const_pool, cc.name_index)
                    .with_context(|_| format_err!("parent of {:?}", cc))?,
                other => Err(format_err!("non-class parent? {:?}", other))?,
            };

            let mut others = HashSet::with_capacity(8);
            for con in &clazz.const_pool {
                match con {
                    ConstantInfo::Class(cc) => {
                        others.insert(
                            as_class_name(&clazz.const_pool, cc.name_index)
                                .with_context(|_| format_err!("other class"))?,
                        );
                    }
                    _ => (),
                }
            }

            others.remove(&parent);
            let mut others = others.into_iter().collect::<Vec<_>>();
            others.sort();
            Ok((parent, others))
        }
        err => Err(format_err!("parse explosion: {:?}", err)),
    }
}

fn as_class_name(const_pool: &[ConstantInfo], index: u16) -> Result<String, Error> {
    match const_pool.get(usize::from(index - 1)) {
        Some(ConstantInfo::Utf8(d)) => Ok(d.utf8_string.to_string()),
        Some(ConstantInfo::NameAndType(nc)) => as_str(const_pool, nc.name_index),
        other => Err(format_err!("non-utf8? {:?}", other)),
    }
}

fn as_str(const_pool: &[ConstantInfo], index: u16) -> Result<String, Error> {
    match const_pool.get(usize::from(index - 1)) {
        Some(ConstantInfo::Utf8(d)) => Ok(d.utf8_string.to_string()),
        other => Err(format_err!("non-utf8? {:?}", other)),
    }
}
