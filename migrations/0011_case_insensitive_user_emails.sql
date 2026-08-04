-- Registration checks emails case-insensitively; enforce the same rule under
-- concurrent requests instead of relying only on the original exact-case key.
CREATE UNIQUE INDEX users_email_lower_idx ON users(lower(email));