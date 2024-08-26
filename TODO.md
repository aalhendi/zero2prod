# TODO

- CI Flowchart
- Clean up README
- Docker Runtime image via Rust Alpine or smaller, compile to linux-musl, rust-musl-builder
- Strip symbols to save size? <https://github.com/johnthagen/min-sized-rust#strip-symbols-from-binary>
- Build something like a spec.yaml manifest for deployments. Inject environment vars
- Update validator crate
- Consider proptest crate for property-based testing.
- Consider switching to figment crate for configurations
- Use a proper templating solution for our emails (e.g. tera);
- What happens if a user clicks on a confirmation link twice?
- What happens if the subscription token is well-formatted but non-existent?
- Add pepper to passwords, follow OWASP

## Milestone Tasks

- Verify properties of logs emitted by application
- Hook OpenObserve
