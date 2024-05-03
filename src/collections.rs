use std::{
    fs::DirEntry,
    io,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail};
use git2::{Cred, FetchOptions, PushOptions, Remote, RemoteCallbacks, Repository, Signature};
use image::ImageFormat;
use rand::seq::IteratorRandom;
use thiserror::Error;

use crate::{image_supplier::SavedImage, BASEDIRECTORIES, CONFIG};

const GIT_REMOTE_NAME: &str = r#"origin"#;

#[derive(Debug, Error)]
pub enum CollectionError {
    #[error("This already exists")]
    AlreadyExists,
    #[error("This cannot be found")]
    NotFound,
    #[error("The name is invalid: {0}")]
    InvalidName(String),
    #[error("The collection does not have a git repository initialized")]
    NoGitFound,
    #[error("A git related failure occured: {0}")]
    GitError(git2::Error),
    #[error("A internal file system error occured: {0}")]
    FsError(io::Error),
    #[error("There are not files to be found in the collection")]
    CollectionEmpty,
}

pub struct CollectionPath(PathBuf);

impl CollectionPath {
    pub fn from_name(name: &str) -> Self {
        Self(BASEDIRECTORIES.get_data_home().join(&name))
    }

    pub fn exists(&self) -> bool {
        self.0.is_dir()
    }

    pub fn create_directory(&self) -> Result<(), CollectionError> {
        match std::fs::DirBuilder::new().create(&self) {
            Ok(_) => Ok(()),
            Err(err) => {
                panic!("{:?}", err);
            }
        }
    }

    pub fn get_image_path(&self, image: &SavedImage) -> Result<PathBuf, CollectionError> {
        let name = match image.get_name() {
            Ok(name) => name,
            Err(_) => panic!("Implement actual error handling here\nImage name not valid UTF8"),
        };

        let format = image.get_format();

        let goal_path = self.0.join(format!(
            "{}.{}",
            name,
            // TODO: Unwrap here, do some error handling here later
            format.extensions_str().first().unwrap()
        ));

        Ok(goal_path)
    }
}

impl AsRef<Path> for CollectionPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

pub struct CollectionRepository(Repository);

impl CollectionRepository {
    fn get_credential_callbacks(&self) -> RemoteCallbacks {
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key(
                username_from_url.unwrap(),
                None,
                Path::new(
                    CONFIG
                        .private_key_path
                        .as_ref()
                        .expect("No SSH key file location specified in config"),
                ),
                None,
            )
        });

        callbacks
    }

    fn get_fetch_options(&self) -> FetchOptions {
        let mut options = FetchOptions::new();
        options.remote_callbacks(self.get_credential_callbacks());
        options
    }

    fn get_push_options(&self) -> PushOptions {
        let mut options = PushOptions::new();
        options.remote_callbacks(self.get_credential_callbacks());
        options
    }

    pub fn initialize<P>(path: P, remote_url: &str) -> Result<Self, CollectionError>
    where
        P: AsRef<Path>,
    {
        // Inner function purely for clean error handling
        fn inner<'b>(path: &Path, remote_url: &str) -> Result<(), git2::Error> {
            let repository = Repository::init(path)?;

            repository.remote(GIT_REMOTE_NAME, &remote_url)?;

            repository
                .index()?
                .add_all(&["."], git2::IndexAddOption::DEFAULT, None)?;

            // Initial commit

            let signature = repository.signature()?;
            let oid = repository.index()?.write_tree()?;
            let tree = repository.find_tree(oid)?;
            repository.commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Initial commit",
                &tree,
                &[],
            )?;

            Ok(())
        }

        match inner(path.as_ref(), remote_url) {
            Ok(_) => {
                let repository = match Repository::open(path) {
                    Ok(repository) => repository,
                    Err(err) => return Err(CollectionError::GitError(err)),
                };

                Ok(Self(repository))
            }
            Err(err) => Err(CollectionError::GitError(err)),
        }
    }

    pub fn open<P>(path: P) -> Result<Self, CollectionError>
    where
        P: AsRef<Path>,
    {
        match Repository::open(path.as_ref()) {
            Ok(repository) => Ok(CollectionRepository(repository)),
            Err(err) if err.code() == git2::ErrorCode::NotFound => Err(CollectionError::NoGitFound),
            Err(err) => return Err(CollectionError::GitError(err)),
        }
    }

    pub fn commit_all(&self, message: &str) -> Result<(), CollectionError> {
        fn inner(repository: &Repository, message: &str) -> Result<(), git2::Error> {
            repository
                .index()?
                .add_all(&["."], git2::IndexAddOption::DEFAULT, None)?;

            let head = repository.head()?;

            let signature = repository.signature()?;
            let oid = repository.index()?.write_tree()?;
            let tree = repository.find_tree(oid)?;
            repository.commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &[&head.peel_to_commit()?],
            )?;

            Ok(())
        }

        inner(&self.0, message).map_err(|err| CollectionError::GitError(err))
    }

    pub fn sync(&self) -> Result<(), CollectionError> {
        fn inner(
            repository: &Repository,
            mut fetch_options: FetchOptions,
            mut push_options: PushOptions,
        ) -> Result<(), git2::Error> {
            let mut remote = repository.find_remote(GIT_REMOTE_NAME)?;

            remote.fetch(&["HEAD"], Some(&mut fetch_options), None)?;

            let main_branch = remote.default_branch()?;
            let main_branch_name = main_branch.as_str().unwrap();
            remote.push(&[main_branch_name], Some(&mut push_options))?;

            Ok(())
        }

        inner(&self.0, self.get_fetch_options(), self.get_push_options())
            .map_err(|err| CollectionError::GitError(err))
    }
}

pub struct CollectionDirectory {
    path: CollectionPath,
    repository: Option<CollectionRepository>,
}

impl CollectionDirectory {
    fn is_collection_name_valid(name: &str) -> Result<(), &'static str> {
        if name.contains(' ') {
            return Err("Name cannot contain spaces");
        }

        if name.to_lowercase() != name {
            return Err("Name must be lowercase");
        }

        Ok(())
    }

    fn check_name(name: &str) -> Result<(), CollectionError> {
        Self::is_collection_name_valid(name)
            .map_err(|reason| CollectionError::InvalidName(reason.to_owned()))
    }

    /// Create a directory
    fn create(name: &str, remote_url: Option<&str>) -> Result<Self, CollectionError> {
        Self::check_name(name)?;

        let path = CollectionPath::from_name(name);
        path.create_directory();

        let repository = match remote_url {
            Some(url) => Some(CollectionRepository::initialize(&path, url)?),
            None => None,
        };

        Ok(Self { path, repository })
    }

    fn open(name: &str) -> Result<Self, CollectionError> {
        Self::check_name(name)?;

        let path = CollectionPath::from_name(name);

        if path.exists() {
            let repository = match CollectionRepository::open(&path) {
                Ok(repository) => Some(repository),
                Err(CollectionError::NoGitFound) => None,
                Err(err) => return Err(err),
            };

            Ok(Self { path, repository })
        } else {
            Err(CollectionError::NotFound)
        }
    }

    fn delete(self) -> Result<(), CollectionError> {
        std::fs::remove_dir_all(self.path).map_err(|err| CollectionError::FsError(err))
    }

    // Getters
    pub fn get_repository(&self) -> Option<&CollectionRepository> {
        self.repository.as_ref()
    }

    pub fn get_repository_mut(&mut self) -> Option<&mut CollectionRepository> {
        self.repository.as_mut()
    }

    pub fn initialize_repository(
        &mut self,
        remote_url: &str,
    ) -> Result<&CollectionRepository, CollectionError> {
        self.repository = Some(CollectionRepository::initialize(&self.path, remote_url)?);

        // We can assure it exists here
        Ok(self.repository.as_ref().unwrap())
    }

    pub fn get_path(&self) -> &CollectionPath {
        &self.path
    }

    // Files

    pub fn get_images(&self) -> Result<Vec<SavedImage>, CollectionError> {
        let files = match self.path.as_ref().read_dir() {
            Ok(files) => files,
            Err(err) => return Err(CollectionError::FsError(err)),
        };

        fn extract_images(entry: Result<DirEntry, io::Error>) -> Option<SavedImage> {
            match entry {
                Ok(entry) if ImageFormat::from_path(entry.path()).is_ok() => {
                    // TODO: Get rid of the unwrap here, and make it return none if the image errored due to an invalid file type
                    Some(SavedImage::from_path(entry.path()).unwrap())
                }
                Ok(_) => None,
                Err(_) => None,
            }
        }

        let images = files.filter_map(extract_images);
        Ok(images.collect())
    }

    pub fn get_random_image(&self) -> Result<SavedImage, CollectionError> {
        // Get images, filter for type and pick a random one
        // TODO: Make the random pick a bit less random and more likely to pick the least used one
        let image = self
            .get_images()?
            .into_iter()
            .choose(&mut rand::thread_rng());

        match image {
            Some(image) => Ok(image),
            None => Err(CollectionError::CollectionEmpty),
        }
    }

    pub fn find_image(&self, name: &str) -> Result<SavedImage, CollectionError> {
        let images = self.get_images()?;
        let image = images
            .into_iter()
            .find(|image| image.get_name().unwrap() == name);

        match image {
            Some(image) => Ok(image),
            None => Err(CollectionError::NotFound),
        }
    }

    pub fn add_image(&self, image: &SavedImage) -> Result<SavedImage, CollectionError> {
        let goal_path = self.path.get_image_path(&image)?;

        match image.copy_to(goal_path) {
            Ok(image) => Ok(image),
            Err(_err) => panic!("Implement actual error handling here\nFailed to copy image"),
        }
    }
}

pub struct Collection {
    name: String,
    directory: CollectionDirectory,
}

impl Collection {
    pub fn create<S>(name: S, remote_url: Option<&str>) -> Result<Self, CollectionError>
    where
        S: ToString,
    {
        let name = name.to_string();
        let directory = CollectionDirectory::create(&name, remote_url)?;

        Ok(Self { name, directory })
    }

    pub fn open<S>(name: S) -> Result<Self, CollectionError>
    where
        S: ToString,
    {
        let name = name.to_string();
        let directory = CollectionDirectory::open(&name)?;

        Ok(Self { name, directory })
    }

    pub fn delete(self) -> Result<(), CollectionError> {
        self.directory.delete()
    }

    pub fn get_directory(&self) -> &CollectionDirectory {
        &self.directory
    }

    pub fn get_directory_mut(&mut self) -> &mut CollectionDirectory {
        &mut self.directory
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_repository(&self) -> Option<&CollectionRepository> {
        self.get_directory().get_repository()
    }
}
