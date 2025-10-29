use wax;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    WaxBuild(#[from] wax::BuildError),
}
