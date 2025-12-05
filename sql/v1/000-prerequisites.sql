-- 000-prerequisites.sql
-- Create recommended PostgreSQL extensions for Heimdall.
--  PREREQUISITE: must be run with full superuser privileges on the database.

CREATE EXTENSION IF NOT EXISTS pgcrypto;


CREATE EXTENSION IF NOT EXISTS pg_trgm;


CREATE EXTENSION IF NOT EXISTS citext;


CREATE EXTENSION IF NOT EXISTS btree_gin;


CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Vector embeddings (pgvector). Extension name is typically 'vector'.

CREATE EXTENSION IF NOT EXISTS vector;

-- Graph extension (Apache AGE). AGE must be installed on the Postgres
-- server binary; enabling the extension requires server support and
-- superuser privileges. We assume AGE is available in your environment.

CREATE EXTENSION IF NOT EXISTS age;

LOAD 'age';


SET search_path = ag_catalog,
    "$user",
    public;