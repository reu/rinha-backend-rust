CREATE EXTENSION pg_trgm;

CREATE OR REPLACE FUNCTION ARRAY_TO_STRING_IMMUTABLE (
  arr TEXT[],
  sep TEXT
) RETURNS TEXT IMMUTABLE PARALLEL SAFE LANGUAGE SQL AS $$
SELECT ARRAY_TO_STRING(arr, sep) $$;

CREATE TABLE people (
  id UUID PRIMARY KEY,
  name VARCHAR(100) NOT NULL,
  nick VARCHAR(32) NOT NULL,
  birth_date DATE NOT NULL,
  stack VARCHAR(32)[],
  search TEXT GENERATED ALWAYS AS (
    name || ' ' || nick || ' ' || COALESCE(ARRAY_TO_STRING_IMMUTABLE(stack, ' '), '')
  ) STORED,
  CONSTRAINT unique_nick UNIQUE (nick)
);

CREATE INDEX people_search_index ON people USING GIST (search gist_trgm_ops);
