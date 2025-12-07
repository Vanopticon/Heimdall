-- Create required extensions in the default database
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS ag; -- Apache AGE

-- Create a schema for AGE and set it up
CREATE SCHEMA IF NOT EXISTS ag_catalog;
SELECT ag_catalog.create_graph('heimdall_graph');
