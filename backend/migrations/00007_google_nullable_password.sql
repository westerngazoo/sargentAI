-- R-0033: Google Sign-In users have no password hash.
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;
