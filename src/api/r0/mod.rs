//! API endpoints for the 0.x.x version of the Matrix spec.

pub use self::login::Login;
pub use self::registration::Register;
pub use self::versions::Versions;

mod login;
mod registration;
mod versions;