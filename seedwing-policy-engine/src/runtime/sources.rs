use crate::lang::ty::PackagePath;
use crate::lang::Source;
use std::fs::File;
use std::io::Read;
use std::iter::{once, Once};
use std::path::{Iter, Path, PathBuf};
use walkdir::WalkDir;

pub struct Ephemeral {
    source: Source,
    content: String,
}

impl Ephemeral {
    pub fn new<P: Into<PackagePath>, C: Into<String>>(package: P, content: C) -> Self {
        Self {
            source: package.into().into(),
            content: content.into(),
        }
    }

    pub fn iter(self) -> impl Iterator<Item = (Source, String)> {
        once((self.source.clone(), self.content))
    }
}

#[derive(Debug)]
pub struct Directory {
    dir: PathBuf,
}

impl Directory {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Source, String)> + '_ {
        WalkDir::new(&self.dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy();
                if !name.ends_with(".dog") {
                    None
                } else {
                    let path = e.path();
                    if let Ok(path) = path.strip_prefix::<&Path>(&self.dir) {
                        let mut src = String::new();
                        if let Some(part) = path.parent() {
                            src.push_str(&*part.to_string_lossy());
                            src.push('/');
                        }
                        src.push_str(name.strip_suffix(".dog").unwrap());
                        if let Ok(mut file) = File::open(e.path()) {
                            let mut content = String::new();
                            file.read_to_string(&mut content);
                            Some((src.into(), content))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            })
    }
}

#[cfg(test)]
mod test {
    use crate::runtime::sources::Directory;
    use std::env;

    #[test]
    fn dir_walking() {
        let dir = Directory::new(env::current_dir().unwrap().join("test-data"));

        for e in dir.iter() {
            println!("--> {:?}", e);
        }
    }
}
