# TODO

- CI Flowchart
- Clean up README
- Build something like a spec.yaml manifest for deployments. Inject environment vars
- Update validator crate
- Consider proptest crate for property-based testing.
- Consider switching to figment crate for configurations
- Use a proper templating solution for our emails (e.g. tera);
- What happens if a user clicks on a confirmation link twice?
- What happens if the subscription token is well-formatted but non-existent?
- Seed admin should be able to invite more collaborators. impl login-protected functionality (subscription flow for inspiration)
- enhance issue_delivery_queue - e.g. adding a n_retries and execute_after

## Milestone Tasks

- Verify properties of logs emitted by application
- Add workflow observability (e.g. Page to track how many emails still outstanding for a newletter issue)
