// No need to define a `main()` function here. Rust test framework adds one behind the scenes.
// Taking full advantage of the fact that each file under tests is its own executable.
// Define submodules scoped to a single executable. Will structure `api` similarly to a binary crate structure.

mod health_check;
mod helpers;
mod newsletter;
mod subscriptions;
mod subscriptions_confirm;
mod login;
