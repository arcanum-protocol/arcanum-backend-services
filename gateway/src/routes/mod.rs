pub mod account;
// pub mod chains;
pub mod charts;
pub mod portfolio;

pub fn stringify<E: ToString>(e: E) -> String {
    e.to_string()
}
