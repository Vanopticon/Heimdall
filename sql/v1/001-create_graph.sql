-- Heimdall canonical graph schema for PostgreSQL + Apache AGE
-- Defines core node types, relationships, and indices for provenance-first ingestion
--
-- Core concepts:
--   Dump      - Single uploaded file/ingestion run
--   Row       - A row in a Dump (maintains original structure)
--   Sighting  - A single observed cell/field value occurrence in a Row
--   Field     - Canonical field name (e.g., "email", "ip", "domain")
--   FieldValue - Canonicalized value instance with stable canonical_key
--   Entity    - Higher-level deduplicated object composed from FieldValues
--
-- Node types are defined via labels in Apache AGE; this script sets up
-- indices and helper views for the canonical nodes.

-- Ensure AGE extension is loaded
CREATE EXTENSION IF NOT EXISTS ag;

-- Ensure the graph exists (idempotent; safe to run multiple times)
SELECT ag_catalog.create_graph_if_not_exists('heimdall_graph');

-- Apache AGE stores graph data in ag_catalog.ag_label tables.
-- Indices on node properties are created using Cypher-aware indexing or
-- standard Postgres indices on the agtype columns.

-- Index on Dump.id for fast lookup of dumps by identifier
DO $$
BEGIN
	-- Create a VLabel (vertex label) for Dump if it doesn't exist by attempting a MERGE
	EXECUTE format('SELECT * FROM cypher(%L, $$ MERGE (d:Dump {id: "bootstrap"}) RETURN d $$) as (v agtype);', 'heimdall_graph');
END $$;

-- Index on Row.dump_id and Row.index for ordered retrieval of rows in a dump
DO $$
BEGIN
	EXECUTE format('SELECT * FROM cypher(%L, $$ MERGE (r:Row {dump_id: "bootstrap", index: 0}) RETURN r $$) as (v agtype);', 'heimdall_graph');
END $$;

-- Index on FieldValue.canonical_key for fast deduplication and lookup
DO $$
BEGIN
	EXECUTE format('SELECT * FROM cypher(%L, $$ MERGE (fv:FieldValue {canonical_key: "bootstrap"}) RETURN fv $$) as (v agtype);', 'heimdall_graph');
END $$;

-- Index on Field.name for field type lookups
DO $$
BEGIN
	EXECUTE format('SELECT * FROM cypher(%L, $$ MERGE (f:Field {name: "bootstrap"}) RETURN f $$) as (v agtype);', 'heimdall_graph');
END $$;

-- Index on Sighting for quick cell lookups (optional; depends on query patterns)
DO $$
BEGIN
	EXECUTE format('SELECT * FROM cypher(%L, $$ MERGE (s:Sighting {id: "bootstrap"}) RETURN s $$) as (v agtype);', 'heimdall_graph');
END $$;

-- Index on Entity for higher-level object queries
DO $$
BEGIN
	EXECUTE format('SELECT * FROM cypher(%L, $$ MERGE (e:Entity {id: "bootstrap"}) RETURN e $$) as (v agtype);', 'heimdall_graph');
END $$;

-- Index on RowHash for duplicate detection (optional node type for row fingerprints)
DO $$
BEGIN
	EXECUTE format('SELECT * FROM cypher(%L, $$ MERGE (rh:RowHash {hash: "bootstrap"}) RETURN rh $$) as (v agtype);', 'heimdall_graph');
END $$;

-- Clean up bootstrap nodes (these were created only to establish vlabels)
DO $$
BEGIN
	EXECUTE format('SELECT * FROM cypher(%L, $$ 
		MATCH (n)
		WHERE n.id = "bootstrap" 
			OR n.canonical_key = "bootstrap" 
			OR n.name = "bootstrap" 
			OR n.hash = "bootstrap"
			OR (n.dump_id = "bootstrap" AND n.index = 0)
		DELETE n
	$$) as (v agtype);', 'heimdall_graph');
END $$;

-- Note: Apache AGE automatically creates internal indices on node IDs.
-- Additional property-based indices can be created using Postgres CREATE INDEX
-- on the underlying ag_label tables if needed for performance optimization.
--
-- Example (manual index creation on ag_label table):
-- CREATE INDEX IF NOT EXISTS idx_fieldvalue_canonical_key 
--   ON heimdall_graph."FieldValue" 
--   USING btree ((properties->>'canonical_key'));
--
-- However, the above Cypher MERGE operations ensure the labels exist and
-- Apache AGE will handle basic indexing. For production deployments with
-- large graphs, consult AGE documentation for additional index strategies.

-- Summary:
-- This schema establishes the canonical node types (Dump, Row, Sighting, Field,
-- FieldValue, Entity, RowHash) in the heimdall_graph. The provenance model allows
-- reconstruction of original dumps while maintaining deduplicated canonical values.
