use seedwing_policy_engine::data::DirectoryDataSource;
use seedwing_policy_engine::error_printer::ErrorPrinter;
use seedwing_policy_engine::lang::builder::Builder;
use seedwing_policy_engine::runtime::sources::Directory;
use seedwing_policy_engine::runtime::World;
use std::path::PathBuf;
use std::process::exit;

pub struct Verify {
    policy_directories: Vec<PathBuf>,
    data_directories: Vec<PathBuf>,
}

impl Verify {
    pub fn new(policy_directories: Vec<PathBuf>, data_directories: Vec<PathBuf>) -> Self {
        Self {
            policy_directories,
            data_directories,
        }
    }

    pub async fn run(&self) -> Result<World, ()> {
        let mut errors = Vec::new();

        let mut builder = Builder::new();
        let mut sources = Vec::new();
        for dir in &self.policy_directories {
            let dir = PathBuf::from(dir);
            if !dir.exists() {
                log::error!("Unable to open directory: {}", dir.to_string_lossy());
                exit(-3);
            }
            sources.push(Directory::new(dir));
        }

        //log::info!("loading policies from {}", dir);
        for source in sources.iter() {
            if let Err(result) = builder.build(source.iter()) {
                errors.extend_from_slice(&result);
            }
        }

        if !errors.is_empty() {
            ErrorPrinter::new(builder.source_cache()).display(&errors);
            exit(-1)
        }

        for each in &self.data_directories {
            log::info!("loading data from {:?}", each);
            builder.data(DirectoryDataSource::new(each.into()));
        }

        let result = builder.finish().await;

        match result {
            Ok(world) => Ok(world),
            Err(errors) => {
                ErrorPrinter::new(builder.source_cache()).display(&errors);
                exit(-2);
            }
        }
    }
}
