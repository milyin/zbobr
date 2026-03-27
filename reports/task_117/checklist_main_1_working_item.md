The reviewer found that `token_auth_env()` still embeds the token in a URL via `url.https://x-access-token:{token}@github.com/.insteadOf`. Git errors/logs may reveal the full remote URL, exposing the credential.

Fix: Replace the `url.*.insteadOf` approach with `http.extraheader` using a base64-encoded Authorization header. This way the token never appears in any URL.

Use two config entries:
- `http.https://github.com/.extraheader` = `Authorization: basic <base64("x-access-token:" + token)>`

This ensures the token is only passed as an HTTP header, never in URLs.