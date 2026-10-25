# WeChat Mini Program client

## Open in WeChat DevTools

1. Import the `wechat/` directory as a Mini Program project.
2. Replace `touristappid` in `project.config.json` with your AppID for real-device testing.
3. Set `baseUrl` in `config.js` to your deployed HTTPS API.
4. Register the API domain under **Development > Development Settings > Server Domains**
   (request domain) before enabling `urlCheck` for production.

## Current auth status

`app.js` calls `POST /auth/wechat` after `wx.login`. That endpoint is **not implemented yet**
(see `docs/SPEC-REVIEW.md`). Until it exists, the client falls back to the seeded demo account
and shows a "Demo account" badge.

## API contract used by this client

- `GET /api/v1/summaries/{user_id}` -> array of six category summaries
- `POST /api/v1/events` -> `201 Created` with the inserted event

The client accepts any 2xx response, so it also works if the backend returns `200 OK`.
