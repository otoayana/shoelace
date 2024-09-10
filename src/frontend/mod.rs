mod formatters;
pub mod routes;
mod templates;

#[allow(unused_imports)]
pub use routes::{post, user};
pub use templates::{Base, ErrorView};
