//! The sel4 build management tool
//!
//! This tool manages repo-based projects for seL4, build tools in podman or docker, configuring
//! sets of build directories, as well as simulating builds and running builds on hardware.
//!
//! The configuration of the tool, which can be extended both from the user's home directory as
//! well as from the project root, can provide information on platforms, projects, hardware
//! runners, and build environments.

mod cmake;
mod config;
mod platform;

pub use cmake::*;
pub use config::*;
pub use platform::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
