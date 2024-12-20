# TODO

- Build something like a spec.yaml manifest for deployments. Inject environment vars
- Consider proptest crate for property-based testing.
- Use a proper templating solution for our emails (e.g. tera);
- What happens if a user clicks on a confirmation link twice?
- What happens if the subscription token is well-formatted but non-existent?
- Seed admin should be able to invite more collaborators. impl login-protected functionality (subscription flow for inspiration)
- enhance issue_delivery_queue - e.g. adding a n_retries and execute_after
- add password reset
- add password expiry
- fix admin hash + document
- docker compose

## Milestone Tasks

- Verify properties of logs emitted by application
- Add workflow observability (e.g. Page to track how many emails still outstanding for a newletter issue)
