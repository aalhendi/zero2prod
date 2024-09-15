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
- Seed admin should be able to invite more collaborators. impl login-protected functionality (subscription flow for inspiration)
- Create middleware wrapping `/admin/` prefix endpoints, checks session state and redirects to `/login` if not logged in.
- Password minimum length [OWASP Link](https://github.com/OWASP/ASVS/blob/master/5.0/en/0x11-V2-Authentication.md).
- enhance issue_delivery_queue - e.g. adding a n_retries and execute_after

## Milestone Tasks

- Verify properties of logs emitted by application
- Hook OpenObserve
- Add workflow observability (e.g. Page to track how many emails still outstanding for a newletter issue)
