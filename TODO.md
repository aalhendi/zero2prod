# TODO

- Build something like a spec.yaml manifest for deployments. Inject environment vars
- Use a proper templating solution for our emails (e.g. tera);
- What happens if a user clicks on a confirmation link twice?
- What happens if the subscription token is well-formatted but non-existent?
- Seed admin should be able to invite more collaborators. impl login-protected functionality (subscription flow for inspiration)
- enhance issue_delivery_queue - e.g. adding a n_retries and execute_after
- password reset confirmation
- password reset confirmation test
- password reset rate limiting
- add password expiry
- docker compose
- refactor migrations into core and seed migrations (so admin user doesn't get seeded into prod env lol)

## Milestone Tasks

- Verify properties of logs emitted by application
- Add workflow observability (e.g. Page to track how many emails still outstanding for a newletter issue)
