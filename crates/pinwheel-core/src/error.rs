/// Every error is written for the person using Pinwheel: it says what is wrong
/// and, where possible, how to fix it.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("\"{0}\" is not a file size. Try something like 300 KB or 1.5 MB.")]
    InvalidSize(String),

    #[error("\"{0}\" is not a pixel size. Try something like 1920x1080.")]
    InvalidDimensions(String),

    #[error("\"{0}\" is not a length. Try something like 35 mm, 3.5 cm or 2 in.")]
    InvalidLength(String),

    #[error("{what} must be between {min} and {max}, not {got}.")]
    OutOfRange {
        what: &'static str,
        min: f64,
        max: f64,
        got: f64,
    },

    #[error("{0}")]
    CannotFit(String),

    #[error("The name template is not valid: {0}")]
    Template(String),

    #[error("This recipe has a problem: {0}")]
    Recipe(String),

    #[error("This spec card has a problem: {0}")]
    Spec(String),
}
