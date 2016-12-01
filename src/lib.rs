extern crate iron;
extern crate route_recognizer as recognizer;
extern crate url;
extern crate mount;

pub mod router;
pub mod url_for;

pub use url_for::url_for;
pub use router::{RouteHandler, Router, RouterError, get_parameter};