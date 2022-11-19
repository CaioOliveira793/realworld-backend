
CREATE SCHEMA iam;

CREATE TABLE iam.user (
  id UUID CONSTRAINT user_pk PRIMARY KEY,
  created TIMESTAMP NOT NULL,
  updated TIMESTAMP,
  version BIGINT NOT NULL,
  username TEXT NOT NULL,
  email TEXT NOT NULL,
  password_hash TEXT NOT NULL,
  bio TEXT,
  image_url TEXT,

  CONSTRAINT user_unique_username UNIQUE (username),
  CONSTRAINT user_unique_email UNIQUE (email)
);

CREATE SCHEMA blog;
