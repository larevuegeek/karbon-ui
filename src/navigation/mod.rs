//! Navigation between the pages of an application: [`History`], the back and forward
//! stacks of a browser tab, for any type of route.
// Organisation : `history` (la pile et ses déplacements).
mod history;
#[cfg(test)]
mod tests;
pub use history::History;
