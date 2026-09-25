import { writeCurrentAuth } from "../src/api/authStorage";

/** Synthetic UI fixture identity; it does not authorize real Server requests. */
export const writeFixtureAuth = (fixture: string): Promise<void> => writeCurrentAuth({
  user: {
    id: 1, username: fixture, email: `${fixture}@example.invalid`, role: "user",
    is_active: true, page_cli_enabled: false, avatar_url: null,
    oauth_provider: null, oauth_subject: null, created_at: null, updated_at: null,
    last_login_at: null,
  },
  tokens: {
    access_token: `${fixture}-access`, refresh_token: `${fixture}-refresh`,
    session_id: fixture, token_type: "Bearer", expires_in: 3600,
  },
  session: {
    id: fixture, created_at: "2026-09-25T00:00:00Z", expires_at: "2026-10-25T00:00:00Z",
    last_used_at: null, user_agent: fixture,
  },
});
