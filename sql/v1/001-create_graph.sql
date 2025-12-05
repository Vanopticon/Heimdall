-- Load the AGE extension into this session so the cypher() function is available.
-- `CREATE EXTENSION age;` must have been run in the database by an admin.
LOAD 'age';


SET search_path = ag_catalog,
    "$user",
    public;

-- Create the primary graph for dumps data only if it does not already exist
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM ag_graph WHERE name = 'dumps_graph') THEN
        PERFORM create_graph('dumps_graph');
    END IF;
END
$$;

-- Ensure a single root node for Dump Sources

SELECT *
FROM cypher('dumps_graph', $$
    MERGE (:sources {name: 'ROOT'})
$$) AS (v agtype);

-- Ensure a single root node for Dumps

SELECT *
FROM cypher('dumps_graph', $$
    MERGE (:dumps {name: 'ROOT'})
$$) AS (v agtype);

-- Ensure a single root node for Fields

SELECT *
FROM cypher('dumps_graph', $$
    MERGE (:fields {name: 'ROOT'})
$$) AS (v agtype);

-- Ensure a single root node for Field Data

SELECT *
FROM cypher('dumps_graph', $$
    MERGE (:field_data {name: 'ROOT'})
$$) AS (v agtype);

-- Ensure a single root node for Rows

SELECT *
FROM cypher('dumps_graph', $$
    MERGE (:rows {name: 'ROOT'})
$$) AS (v agtype);

-- Ensure a single root node for Field Values

SELECT *
FROM cypher('dumps_graph', $$
    MERGE (:field_value {name: 'ROOT'})
$$) AS (v agtype);

-- Create nodes for Non-Public Information (NPI) categories

SELECT *
FROM cypher('dumps_graph', $$
    // ensure a single root node for NPI categories
    MERGE (:NPI_Category {name: 'ROOT'})
$$) AS (v agtype);


SELECT *
FROM cypher('dumps_graph', $$
    // ensure each subcategory exists (idempotent)
    MERGE (:NPI_Category {name: 'IDENTIFICATION'})
    MERGE (:NPI_Category {name: 'AAA'})
    MERGE (:NPI_Category {name: 'PII'})
    MERGE (:NPI_Category {name: 'FINANCIAL'})
    MERGE (:NPI_Category {name: 'HEALTH'})
    MERGE (:NPI_Category {name: 'EMPLOYMENT'})
    MERGE (:NPI_Category {name: 'BEHAVIORAL'})
    MERGE (:NPI_Category {name: 'INFRASTRUCTURE'})
    MERGE (:NPI_Category {name: 'COMMUNICATIONS'})
    MERGE (:NPI_Category {name: 'OTHER_NPI'})
$$) AS (v agtype);


SELECT *
FROM cypher('dumps_graph', $$
    // link the root node to each named subcategory (idempotent)
    MATCH (root:NPI_Category {name: 'ROOT'}), (c:NPI_Category)
    WHERE c.name IN ['IDENTIFICATION', 'AAA', 'PII', 'FINANCIAL', 'HEALTH', 'EMPLOYMENT', 'BEHAVIORAL', 'INFRASTRUCTURE', 'COMMUNICATIONS', 'OTHER_NPI']
    MERGE (root)-[:HAS_SUBCATEGORY]->(c)
$$) AS (v agtype);

-- Ensure a single root node for Sightings

SELECT *
FROM cypher('dumps_graph', $$
    MERGE (:sightings {name: 'ROOT'})
$$) AS (v agtype);