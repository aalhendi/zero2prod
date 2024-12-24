// No need to define a `main()` function here. Rust test framework adds one behind the scenes.
// Taking full advantage of the fact that each file under tests is its own executable.
// Define submodules scoped to a single executable. Will structure `api` similarly to a binary crate structure.

mod admin_dashboard;
mod change_password;
mod health_check;
mod helpers;
mod login;
mod newsletter;
mod subscriptions;
mod subscriptions_confirm;
mod forgot_password;
